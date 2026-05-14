// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// Auto generated from pyronyx-gen — generated extensions
// Do not Edit! Execute `cargo run pyronyx-gen`
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

use crate::vk::*;
use core::ffi::CStr;

/// Type: `Device`
pub const NAME: &CStr = c"VK_ARM_scheduling_controls";
pub const SPEC_VERSION: u32 = 2;

pub trait SchedulingControlsCommandBuffer {
    fn set_dispatch_parameters(&self, dispatch_parameters: &DispatchParametersARM);
}

impl SchedulingControlsCommandBuffer for CommandBuffer {
    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkCmdSetDispatchParametersARM.html>
    ///
    /// Queues types: `Compute`.
    /// Task: `Vulkan state access`.
    /// Use outside `RenderPass`.
    /// Command buffer level: `primary`, `secondary`.
    #[inline]
    fn set_dispatch_parameters(&self, dispatch_parameters: &DispatchParametersARM) {
        let call = self
            .fns()
            .arm_scheduling_controls
            .as_ref()
            .expect(Self::EXT_LOAD_ERROR)
            .set_dispatch_parameters_arm;

        unsafe { (call)(self.handle, dispatch_parameters) };
    }
}
