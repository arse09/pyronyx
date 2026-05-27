// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// Auto generated from pyronyx-gen — generated extensions
// Do not Edit! Execute `cargo run pyronyx-gen`
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

use crate::vk::*;
use core::ffi::CStr;

/// Type: `Device`
pub const NAME: &CStr = c"VK_EXT_primitive_restart_index";
pub const SPEC_VERSION: u32 = 1;

pub trait PrimitiveRestartIndexCommandBuffer {
    fn set_primitive_restart_index(&self, primitive_restart_index: u32);
}

impl PrimitiveRestartIndexCommandBuffer for CommandBuffer {
    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdSetPrimitiveRestartIndexEXT.html>
    ///
    /// Queues types: `Graphics`.
    /// Task: `Vulkan state access`.
    /// Use inside and outside `RenderPass`.
    /// Command buffer level: `primary`, `secondary`.
    #[inline]
    fn set_primitive_restart_index(&self, primitive_restart_index: u32) {
        let call = self
            .fns()
            .ext_primitive_restart_index
            .as_ref()
            .expect(Self::EXT_LOAD_ERROR)
            .set_primitive_restart_index_ext;

        unsafe { (call)(self.handle, primitive_restart_index) };
    }
}
