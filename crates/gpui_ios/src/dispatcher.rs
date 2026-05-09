use super::*;

pub(crate) struct IosDispatcher;

impl IosDispatcher {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn run_runnable(runnable: RunnableVariant) {
        let metadata = runnable.metadata();

        let location = metadata.location;
        let start = std::time::Instant::now();
        let timing = TaskTiming {
            location,
            start,
            end: None,
        };

        THREAD_TIMINGS.with(|timings| {
            let mut timings = timings.lock();
            let timings = &mut timings.timings;
            if let Some(last_timing) = timings.iter_mut().rev().next() {
                if last_timing.location == timing.location {
                    return;
                }
            }
            timings.push_back(timing);
        });

        runnable.run();
        let end = std::time::Instant::now();
        THREAD_TIMINGS.with(|timings| {
            let mut timings = timings.lock();
            let timings = &mut timings.timings;
            if let Some(last_timing) = timings.iter_mut().rev().next() {
                last_timing.end = Some(end);
            }
        });
    }

    pub(crate) fn dispatch_get_main_queue() -> DispatchQueue {
        dispatch_get_main_queue_ptr()
    }

    pub(crate) fn queue_priority(priority: Priority) -> isize {
        match priority {
            Priority::RealtimeAudio => {
                panic!("RealtimeAudio priority should use spawn_realtime, not dispatch")
            }
            Priority::High => DISPATCH_QUEUE_PRIORITY_HIGH,
            Priority::Medium => DISPATCH_QUEUE_PRIORITY_DEFAULT,
            Priority::Low => DISPATCH_QUEUE_PRIORITY_LOW,
        }
    }

    pub(crate) fn duration_to_dispatch_delta(duration: Duration) -> i64 {
        let nanos = duration.as_nanos();
        if nanos > i64::MAX as u128 {
            i64::MAX
        } else {
            nanos as i64
        }
    }
}

pub(crate) fn dispatch_get_main_queue_ptr() -> DispatchQueue {
    addr_of!(_dispatch_main_q) as *const _ as DispatchQueue
}

extern "C" fn dispatch_trampoline(context: *mut c_void) {
    let runnable =
        unsafe { RunnableVariant::from_raw(NonNull::new_unchecked(context.cast::<()>())) };
    IosDispatcher::run_runnable(runnable);
}

impl PlatformDispatcher for IosDispatcher {
    fn get_all_timings(&self) -> Vec<ThreadTaskTimings> {
        let global_timings = GLOBAL_THREAD_TIMINGS.lock();
        ThreadTaskTimings::convert(&global_timings)
    }

    fn get_current_thread_timings(&self) -> ThreadTaskTimings {
        THREAD_TIMINGS.with(|timings| {
            let timings = timings.lock();
            let raw_timings = &timings.timings;
            let mut vec = Vec::with_capacity(raw_timings.len());
            let (s1, s2) = raw_timings.as_slices();
            vec.extend_from_slice(s1);
            vec.extend_from_slice(s2);
            ThreadTaskTimings {
                thread_name: timings.thread_name.clone(),
                thread_id: timings.thread_id,
                timings: vec,
                total_pushed: timings.total_pushed,
            }
        })
    }

    fn is_main_thread(&self) -> bool {
        unsafe {
            let result: objc::runtime::BOOL = msg_send![class!(NSThread), isMainThread];
            result != objc::runtime::NO
        }
    }

    fn dispatch(&self, runnable: RunnableVariant, priority: Priority) {
        let context = runnable.into_raw().as_ptr() as *mut c_void;
        let queue_priority = Self::queue_priority(priority);
        unsafe {
            dispatch_async_f(
                dispatch_get_global_queue(queue_priority, 0),
                context,
                Some(dispatch_trampoline),
            );
        }
    }

    fn dispatch_on_main_thread(&self, runnable: RunnableVariant, _priority: Priority) {
        let context = runnable.into_raw().as_ptr() as *mut c_void;
        unsafe {
            dispatch_async_f(
                Self::dispatch_get_main_queue(),
                context,
                Some(dispatch_trampoline),
            );
        }
    }

    fn dispatch_after(&self, duration: Duration, runnable: RunnableVariant) {
        let context = runnable.into_raw().as_ptr() as *mut c_void;
        let delta = Self::duration_to_dispatch_delta(duration);
        unsafe {
            let when = dispatch_time(DISPATCH_TIME_NOW, delta);
            dispatch_after_f(
                when,
                dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_HIGH, 0),
                context,
                Some(dispatch_trampoline),
            );
        }
    }

    fn spawn_realtime(&self, f: Box<dyn FnOnce() + Send>) {
        let _ = thread::Builder::new()
            .name("gpui-ios-realtime".into())
            .spawn(f);
    }
}
