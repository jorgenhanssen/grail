use crate::extensions::check_extension;

use super::Engine;

impl Engine {
    #[inline(always)]
    pub(super) fn get_extension(&self, gives_check: bool, is_pv_node: bool) -> u8 {
        check_extension(gives_check, is_pv_node)
    }
}
