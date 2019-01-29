use crate::access_port::AccessPort;
use crate::access_port::AccessPortNumber;
use std::collections::HashMap;
use dbg_probe::{
    protocol::WireProtocol,
    debug_probe::{
        DebugProbeError,
        DebugProbe,
    },
};

pub enum DebugPortError {
    DebugProbeError(DebugProbeError),
}

impl DebugPortError {
    pub fn from(error: DebugProbeError) -> DebugPortError {
        DebugPortError::DebugProbeError(error)
    }
}

pub trait DebugPort : DebugProbe {
    // DP register addresses.
    const DP_IDCODE: u32 = 0x0; // read-only
    const DP_ABORT: u32 = 0x0; // write-only
    const DP_CTRL_STAT: u32 = 0x4; // read-write
    const DP_SELECT: u32 = 0x8; // write-only
    const DP_RDBUFF: u32 = 0xC; // read-only

    const ABORT_STKERRCLR: u32 = 0x00000004;

    // DP Control / Status Register bit definitions
    const CTRLSTAT_STICKYORUN: u32 = 0x00000002;
    const CTRLSTAT_STICKYCMP: u32 = 0x00000010;
    const CTRLSTAT_STICKYERR: u32 = 0x00000020;

    const DPIDR_MIN_MASK: u32 = 0x10000;
    const DPIDR_VERSION_MASK: u32 = 0xf000;
    const DPIDR_VERSION_SHIFT: u32 = 12;

    const CSYSPWRUPACK: u32 = 0x80000000;
    const CDBGPWRUPACK: u32 = 0x20000000;
    const CSYSPWRUPREQ: u32 = 0x40000000;
    const CDBGPWRUPREQ: u32 = 0x10000000;

    const TRNNORMAL: u32 = 0x00000000;
    const MASKLANE: u32 = 0x00000f00;

    /// Sets the frequency for JTAG and SWD in Hz.
    fn set_clock(self, frequency: usize);

    /// Initailize DAP IO pins for JTAG or SWD
    fn connect(&mut self) -> Result<(), DebugPortError>;

    /// Deinitialize the DAP I/O pins
    fn disconnect(&mut self)-> Result<(), DebugPortError>;
    
    /// Returns True if the target reset line is asserted or False if de-asserted
    fn is_reset_asserted(&mut self) -> Result<bool, DebugPortError>;

    fn read_ap_multiple(&mut self, addr: u32, count: u8) -> Result<Vec<u8>, DebugPortError>;

    fn write_ap_multiple(&mut self, addr: u32, values: Vec<u8>);
    
    fn get_memory_interface_for_ap(&mut self, apsel: AccessPortNumber);

    fn get_access_ports(&mut self) -> HashMap<AccessPortNumber, Box<AccessPort>>;

    /// Connect to the target.
    fn init(&mut self, wire_protocol: WireProtocol) -> Result<(), DebugPortError> {
        self.set_wire_protocol(wire_protocol);
        self.connect()?;
        self.swj_sequence()?;
        if let Err(TransferError) = self.read_id_code() {
            // If the read of the DP IDCODE fails, retry SWJ sequence. The DP may have been
            // in a state where it thought the SWJ sequence was an invalid transfer.
            self.swj_sequence()?;
            self.read_id_code()?;
        }
        self.clear_sticky_err()
    }

    /// Read ID register and get DebugPort version
    fn read_id_code(&mut self) -> Result<u32, DebugPortError> {
        let dpidr = self.read_reg(Self::DP_IDCODE)?;
        let _dp_version = (self.dpidr & Self::DPIDR_VERSION_MASK) >> Self::DPIDR_VERSION_SHIFT;
        let _is_mindp = (self.dpidr & Self::DPIDR_MIN_MASK) != 0;
        dpidr
    }

    /// Needs to call `self.handle_error()` on the Result.
    fn flush(&mut self) -> Result<(), DebugPortError>;

    fn read_reg(&mut self, addr: u32) -> Result<u32, DebugPortError> {
        self.read_dp(addr)
    }

    fn write_reg(&mut self, addr: u32, value: u32) -> Result<(), DebugPortError> {
        self.write_dp(addr, value)
    }

    fn power_up_debug(&mut self) -> Result<(), DebugPortError> {
        // select bank 0 (to access DRW and TAR)
        self.write_reg(DebugPort::DP_SELECT, 0)?;
        self.write_reg(DebugPort::DP_CTRL_STAT, DebugPort::CSYSPWRUPREQ | DebugPort::CDBGPWRUPREQ)?;

        loop {
            let value = self.read_reg(DebugPort::DP_CTRL_STAT);
            if let Ok(r) = value {
                if (r & (DebugPort::CDBGPWRUPACK | DebugPort::CSYSPWRUPACK)) == (DebugPort::CDBGPWRUPACK | DebugPort::CSYSPWRUPACK) {
                    break;
                }
            } else {
                return value.map(|_| ());
            }
        }

        self.write_reg(DebugPort::DP_CTRL_STAT, DebugPort::CSYSPWRUPREQ | DebugPort::CDBGPWRUPREQ | DebugPort::TRNNORMAL | DebugPort::MASKLANE)?;
        self.write_reg(DebugPort::DP_SELECT, 0)
    }

    fn power_down_debug(&mut self) -> Result<(), DebugPortError> {
        // select bank 0 (to access DRW and TAR)
        self.write_reg(DebugPort::DP_SELECT, 0)?;
        self.write_reg(DebugPort::DP_CTRL_STAT, 0)
    }

    /// Reset the target
    fn reset_all(&mut self) -> Result<(), DebugPortError> {
        for ap in self.get_access_ports().values() {
            ap.reset_did_occur()?;
        }
        self.reset()
    }

    /// Assert or de-assert target reset line
    fn assert_reset_all(&mut self, assert: bool) -> Result<(), DebugPortError> {
        if assert {
            for ap in self.get_access_ports().values() {
                ap.reset_did_occur()?;
            }
        }
        self.assert_reset(assert)
    }

    /// Find valid APs.
    ///
    /// Scans for valid APs starting at APSEL=0 and stopping the first time a 0 is returned
    /// when reading the AP's IDR.
    /// 
    /// Note that a few MCUs will lock up when accessing invalid APs. Those MCUs will have to
    /// modify the init call sequence to substitute a fixed list of valid APs. In fact, that
    /// is a major reason this method is separated from create_aps().
    fn find_aps(&mut self) -> Result<Vec<AccessPortNumber>, DebugPortError> {
        let valid_aps = vec![];
        let mut ap_num: AccessPortNumber = 0;
        loop {
            match AccessPort::probe(self, ap_num) {
                Ok(_) => valid_aps.push(ap_num),
                Err(InvalidAccessPort) => (),
                e => return e
            }
            ap_num += 1;
        }
    }

    /// Init task to create a single AP object.
    fn create_ap(&mut self, access_port: AccessPortNumber) -> Result<(), DebugPortError> {
        let ap = AccessPort::create(&self, access_port)?;
        self.get_access_ports().insert(access_port, ap);
        Ok(())
    }

    /// Needs to call `self.handle_error()` on the Result.
    fn read_dp(&mut self, addr: u32) -> Result<u32, DebugPortError>;

    /// Needs to call `self.handle_error()` on the Result.
    fn write_dp(&mut self, addr: u32, data: u32) -> Result<(), DebugPortError>;

    /// Needs to call `self.handle_error()` on the Result.
    fn write_ap(&mut self, addr: u32, data: u32) -> Result<(), DebugPortError>;

    /// Needs to call `self.handle_error()` on the Result.
    fn read_ap(&mut self, addr: u32) -> Result<u32, DebugPortError>;

    fn handle_error(&mut self, error: DebugProbeError) -> Result<(), DebugPortError> {
        if let DebugProbeError::TransferFault = error {
            self.clear_sticky_err()?;
        }
        Err(DebugPortError::from(error))
    }

    fn clear_sticky_err(&mut self) -> Result<(), DebugPortError> {
        match self.get_wire_protocol() {
            WireProtocol::Swd => self.write_reg(DebugPort::DP_ABORT, DebugPort::ABORT_STKERRCLR),
            WireProtocol::JTag => self.write_reg(DebugPort::DP_CTRL_STAT, DebugPort::CTRLSTAT_STICKYERR),
        }
    }
}

// TODO: Impl those functions soon.

    // ## @brief Init task that returns a call sequence to create APs.
    // #
    // # For each AP in the #valid_aps list, an AccessPort object is created. The new objects
    // # are added to the #aps dict, keyed by their AP number.
    // def create_aps(self):
    //     seq = CallSequence()
    //     for ap_num in self.valid_aps:
    //         seq.append(
    //             ('create_ap.{}'.format(ap_num), lambda ap_num=ap_num: self.create_1_ap(ap_num))
    //             )
    //     return seq
    
    // ## @brief Init task that generates a call sequence to init all AP ROMs.
    // def init_ap_roms(self):
    //     seq = CallSequence()
    //     for ap in [x for x in self.aps.values() if x.has_rom_table]:
    //         seq.append(
    //             ('init_ap.{}'.format(ap.ap_num), ap.init_rom_table)
    //             )
    //     return seq