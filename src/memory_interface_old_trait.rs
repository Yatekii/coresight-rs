use ssmarshal;

// TODO: Write tests (they are really easy to do here!)

/// Interface for memory access.
pub trait MemoryInterface {

    /// Write a single memory location.
    /// 
    /// By default the transfer size is a word.
    fn write_memory(&mut self, addr: u32, data: &[u8]);
        
    /// Read a memory location.
    ///
    /// By default, a word will be read.
    fn read_memory(&mut self, addr: u32, data: &mut [u8]);

    /// Write an aligned block of 32-bit words.
    fn write_memory_block32(&mut self, addr: u32, data: &[u32]);

    /// Read an aligned block of 32-bit words.
    fn read_memory_block32(&mut self, addr: u32, data: &mut [u32]);
  
    /// Shorthand to write a 32-bit word.
    fn write32(&mut self, addr: u32, value: u32) {
        let mut buf = [0; 4];
        ssmarshal::serialize(&mut buf, &value);
        self.write_memory(addr, &buf);
    }

    /// Shorthand to write a 16-bit halfword.
    fn write16(&mut self, addr: u32, value: u16) {
        let mut buf = [0; 2];
        ssmarshal::serialize(&mut buf, &value);
        self.write_memory(addr, &buf);
    }

    /// Shorthand to write a byte.
    fn write8(&mut self, addr: u32, value: u8) {
        let mut buf = [0; 1];
        ssmarshal::serialize(&mut buf, &value);
        self.write_memory(addr, &buf);
    }

    /// Shorthand to read a 32-bit word.
    fn read32(&mut self, addr: u32) -> u32 {
        let mut buf = [0; 4];
        self.read_memory(addr, &mut buf);
        ssmarshal::deserialize(&buf).unwrap().0
    }

    /// Shorthand to read a 16-bit halfword.
    fn read16(&mut self, addr: u32) -> u16 {
        let mut buf = [0; 2];
        self.read_memory(addr, &mut buf);
        ssmarshal::deserialize(&buf).unwrap().0
    }

    /// Shorthand to read a byte.
    fn read8(&mut self, addr: u32) -> u8 {
        let mut buf = [0; 1];
        self.read_memory(addr, &mut buf);
        ssmarshal::deserialize(&buf).unwrap().0
    }

    /// Read a block of unaligned bytes in memory.
    fn read_memory_block8(&mut self, mut addr: u32, mut size: u32) -> Vec<u8> {
        let mut res = vec![];
        // try to read 8bits data
        if (size > 0) && (addr & 0x01 > 0) {
            let mem = self.read8(addr);
            res.push(mem);
            size -= 1;
            addr += 1;
        }
        // try to read 16bits data
        if (size > 1) && (addr & 0x02 > 0) {
            let mem = self.read16(addr);
            let mut buf = [0; 2];
            ssmarshal::serialize(&mut buf, &mem);
            res.extend(&buf);
            size -= 2;
            addr += 2;
        }

        // try to read aligned block of 32bits
        if size >= 4 {
            let mut v = Vec::with_capacity(size as usize);
            self.read_memory_block32(addr, v.as_mut_slice());
            let mut buf = [0; 4];
            v.iter().for_each(|i| {
                ssmarshal::serialize(&mut buf, &i);
                res.extend(&buf);
                size -= 4;
                addr += 4;
            });
        }

        if size > 1 {
            let mem = self.read16(addr);
            let mut buf = [0; 2];
            ssmarshal::serialize(&mut buf, &mem);
            res.extend(&buf);
            size -= 2;
            addr += 2;
        }

        if size > 0 {
            let mem = self.read8(addr);
            res.push(mem);
        }

        res
    }

    /// brief Write a block of unaligned bytes in memory.
    fn write_memory_block8(&mut self, addr: u32, data: Vec<u8>) {
        let size = data.len();
        let idx = 0;

        // try to write 8 bits data
        if (size > 0) && (addr & 0x01 > 0) {
            self.write8(addr, data[idx]);
            size -= 1;
            addr += 1;
            idx += 1;
        }

        // try to write 16 bits data
        if (size > 1) && (addr & 0x02 > 0) {
            self.write16(addr, data[idx] as u16 | ((data[idx + 1] as u16) << 8));
            size -= 2;
            addr += 2;
            idx += 2;
        }

        // write aligned block of 32 bits
        if size >= 4 {
            let mut buf = &data[idx..idx + (size & !0x03)];
            let mut i = 0;
            let mut value = 0;
            let mut v = Vec::with_capacity(size & !0x03);
            while i < buf.len() {
                ssmarshal::serialize(&mut buf[i..i + 4], &mut value);
                v.push(value);
            }
            self.write_memory_block32(addr, v.as_slice());
            
            addr += (size & !0x03) as u32;
            idx += (size & !0x03) as usize;
            size -= size & !0x03;
        }

        // try to write 16 bits data
        if size > 1 {
            self.write16(addr, data[idx] as u16 | ((data[idx + 1] as u16) << 8));
            size -= 2;
            addr += 2;
            idx += 2;
        }

        // try to write 8 bits data
        if size > 0 {
            self.write8(addr, data[idx]);
        }
    }
}