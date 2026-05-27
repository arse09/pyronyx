// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// Auto generated from pyronyx-gen — generated extensions
// Do not Edit! Execute `cargo run pyronyx-gen`
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

use crate::vk::*;
use core::ffi::CStr;

/// Type: `Device`
pub const NAME: &CStr = c"VK_QCOM_queue_perf_hint";
pub const SPEC_VERSION: u32 = 1;

pub trait QueuePerfHintQueue {
    fn set_perf_hint(&self, perf_hint_info: &PerfHintInfoQCOM) -> Result<()>;
}

impl QueuePerfHintQueue for Queue {
    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkQueueSetPerfHintQCOM.html>
    #[inline]
    fn set_perf_hint(&self, perf_hint_info: &PerfHintInfoQCOM) -> Result<()> {
        let call = self
            .fns()
            .qcom_queue_perf_hint
            .as_ref()
            .expect(Self::EXT_LOAD_ERROR)
            .queue_set_perf_hint_qcom;

        unsafe { (call)(self.handle, perf_hint_info) }.result()
    }
}
