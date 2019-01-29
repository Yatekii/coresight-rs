pub trait Component {
    fn ap(&self);
    fn get_cmpid(&self);
    fn set_cmpid(self, component_id: u32);
    fn get_address(self);
    fn set_address(self, address: u32);
}