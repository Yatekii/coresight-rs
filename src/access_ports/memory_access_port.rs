use crate::access_port::AccessPort;

pub struct MemoryAccessPort;

// class MEM_AP(AccessPort, memory_interface.MemoryInterface):
//     def __init__(self, dp, ap_num):
//         super(MEM_AP, self).__init__(dp, ap_num)
        
//         ## Cached CSW value.
//         self._csw = -1

//         # Default to the smallest size supported by all targets.
//         # A size smaller than the supported size will decrease performance
//         # due to the extra address writes, but will not create any
//         # read/write errors.
//         self.auto_increment_page_size = 0x400
        
//         # Ask the probe for an accelerated memory interface for this AP. If it provides one,
//         # then bind our memory interface APIs to its methods. Otherwise use our standard
//         # memory interface based on AP register accesses.
//         memoryInterface = self.dp.link.get_memory_interface_for_ap(self.ap_num)
//         if memoryInterface is not None:
//             logging.debug("Using accelerated memory access interface")
//             self.write_memory = memoryInterface.write_memory
//             self.read_memory = memoryInterface.read_memory
//             self.write_memory_block32 = memoryInterface.write_memory_block32
//             self.read_memory_block32 = memoryInterface.read_memory_block32
//         else:
//             self.write_memory = self._write_memory
//             self.read_memory = self._read_memory
//             self.write_memory_block32 = self._write_memory_block32
//             self.read_memory_block32 = self._read_memory_block32

//     def read_reg(self, addr, now=True):
//         ap_regaddr = addr & APREG_MASK
//         if ap_regaddr == MEM_AP_CSW and self._csw != -1 and now:
//             return self._csw
//         return super(MEM_AP, self).read_reg(addr, now)

//     def write_reg(self, addr, data):
//         ap_regaddr = addr & APREG_MASK

//         # Don't need to write CSW if it's not changing value.
//         if ap_regaddr == MEM_AP_CSW:
//             if data == self._csw:
//                 if LOG_DAP:
//                     num = self.dp.next_access_number
//                     self.logger.info("write_ap:%06d cached (addr=0x%08x) = 0x%08x", num, addr, data)
//                 return
//             self._csw = data

//         try:
//             super(MEM_AP, self).write_reg(addr, data)
//         except exceptions.ProbeError:
//             # Invalidate cached CSW on exception.
//             if ap_regaddr == MEM_AP_CSW:
//                 self._csw = -1
//             raise
    
//     def reset_did_occur(self):
//         # TODO use notifications to invalidate CSW cache.
//         self._csw = -1

//     ## @brief Write a single memory location.
//     #
//     # By default the transfer size is a word
//     def _write_memory(self, addr, data, transfer_size=32):
//         assert (addr & (transfer_size // 8 - 1)) == 0
//         num = self.dp.next_access_number
//         if LOG_DAP:
//             self.logger.info("write_mem:%06d (addr=0x%08x, size=%d) = 0x%08x {", num, addr, transfer_size, data)
//         self.write_reg(MEM_AP_CSW, CSW_VALUE | TRANSFER_SIZE[transfer_size])
//         if transfer_size == 8:
//             data = data << ((addr & 0x03) << 3)
//         elif transfer_size == 16:
//             data = data << ((addr & 0x02) << 3)

//         try:
//             self.write_reg(MEM_AP_TAR, addr)
//             self.write_reg(MEM_AP_DRW, data)
//         except exceptions.TransferFaultError as error:
//             # Annotate error with target address.
//             self._handle_error(error, num)
//             error.fault_address = addr
//             error.fault_length = transfer_size // 8
//             raise
//         except exceptions.Error as error:
//             self._handle_error(error, num)
//             raise
//         if LOG_DAP:
//             self.logger.info("write_mem:%06d }", num)

//     ## @brief Read a memory location.
//     #
//     # By default, a word will be read.
//     def _read_memory(self, addr, transfer_size=32, now=True):
//         assert (addr & (transfer_size // 8 - 1)) == 0
//         num = self.dp.next_access_number
//         if LOG_DAP:
//             self.logger.info("read_mem:%06d (addr=0x%08x, size=%d) {", num, addr, transfer_size)
//         res = None
//         try:
//             self.write_reg(MEM_AP_CSW, CSW_VALUE | TRANSFER_SIZE[transfer_size])
//             self.write_reg(MEM_AP_TAR, addr)
//             result_cb = self.read_reg(MEM_AP_DRW, now=False)
//         except exceptions.TransferFaultError as error:
//             # Annotate error with target address.
//             self._handle_error(error, num)
//             error.fault_address = addr
//             error.fault_length = transfer_size // 8
//             raise
//         except exceptions.Error as error:
//             self._handle_error(error, num)
//             raise

//         def read_mem_cb():
//             try:
//                 res = result_cb()
//                 if transfer_size == 8:
//                     res = (res >> ((addr & 0x03) << 3) & 0xff)
//                 elif transfer_size == 16:
//                     res = (res >> ((addr & 0x02) << 3) & 0xffff)
//                 if LOG_DAP:
//                     self.logger.info("read_mem:%06d %s(addr=0x%08x, size=%d) -> 0x%08x }", num, "" if now else "...", addr, transfer_size, res)
//             except exceptions.TransferFaultError as error:
//                 # Annotate error with target address.
//                 self._handle_error(error, num)
//                 error.fault_address = addr
//                 error.fault_length = transfer_size // 8
//                 raise
//             except exceptions.Error as error:
//                 self._handle_error(error, num)
//                 raise
//             return res

//         if now:
//             result = read_mem_cb()
//             return result
//         else:
//             return read_mem_cb

//     ## @brief Write a single transaction's worth of aligned words.
//     #
//     # The transaction must not cross the MEM-AP's auto-increment boundary.
//     def _write_block32(self, addr, data):
//         assert (addr & 0x3) == 0
//         num = self.dp.next_access_number
//         if LOG_DAP:
//             self.logger.info("_write_block32:%06d (addr=0x%08x, size=%d) {", num, addr, len(data))
//         # put address in TAR
//         self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)
//         self.write_reg(MEM_AP_TAR, addr)
//         try:
//             self.link.write_ap_multiple((self.ap_num << APSEL_SHIFT) | MEM_AP_DRW, data)
//         except exceptions.TransferFaultError as error:
//             # Annotate error with target address.
//             self._handle_error(error, num)
//             error.fault_address = addr
//             error.fault_length = len(data) * 4
//             raise
//         except exceptions.Error as error:
//             self._handle_error(error, num)
//             raise
//         if LOG_DAP:
//             self.logger.info("_write_block32:%06d }", num)

//     ## @brief Read a single transaction's worth of aligned words.
//     #
//     # The transaction must not cross the MEM-AP's auto-increment boundary.
//     def _read_block32(self, addr, size):
//         assert (addr & 0x3) == 0
//         num = self.dp.next_access_number
//         if LOG_DAP:
//             self.logger.info("_read_block32:%06d (addr=0x%08x, size=%d) {", num, addr, size)
//         # put address in TAR
//         self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)
//         self.write_reg(MEM_AP_TAR, addr)
//         try:
//             resp = self.link.read_ap_multiple((self.ap_num << APSEL_SHIFT) | MEM_AP_DRW, size)
//         except exceptions.TransferFaultError as error:
//             # Annotate error with target address.
//             self._handle_error(error, num)
//             error.fault_address = addr
//             error.fault_length = size * 4
//             raise
//         except exceptions.Error as error:
//             self._handle_error(error, num)
//             raise
//         if LOG_DAP:
//             self.logger.info("_read_block32:%06d }", num)
//         return resp

//     ## @brief Write a block of aligned words in memory.
//     def _write_memory_block32(self, addr, data):
//         assert (addr & 0x3) == 0
//         size = len(data)
//         while size > 0:
//             n = self.auto_increment_page_size - (addr & (self.auto_increment_page_size - 1))
//             if size*4 < n:
//                 n = (size*4) & 0xfffffffc
//             self._write_block32(addr, data[:n//4])
//             data = data[n//4:]
//             size -= n//4
//             addr += n
//         return

//     ## @brief Read a block of aligned words in memory.
//     #
//     # @return An array of word values
//     def _read_memory_block32(self, addr, size):
//         assert (addr & 0x3) == 0
//         resp = []
//         while size > 0:
//             n = self.auto_increment_page_size - (addr & (self.auto_increment_page_size - 1))
//             if size*4 < n:
//                 n = (size*4) & 0xfffffffc
//             resp += self._read_block32(addr, n//4)
//             size -= n//4
//             addr += n
//         return resp

//     def _handle_error(self, error, num):
//         self.dp._handle_error(error, num)
//         self._csw = -1

impl AccessPort for MemoryAccessPort {

}