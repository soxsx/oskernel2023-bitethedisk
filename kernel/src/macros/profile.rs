#[macro_export]
macro_rules! time {
    ($title:literal $($piece:tt)*) => {
        let timestamp = crate::timer::get_timeval();
        $($piece)*
        info!("[{}] time cost: {}", $title, crate::timer::get_timeval() - timestamp);
    };
}
