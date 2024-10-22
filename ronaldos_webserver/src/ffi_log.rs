use core::ffi;
use core::ffi::CStr;
use paste::paste;

macro_rules! declare_log_ffi {
    ($($name: ident),+) => {
        $(
            paste! {
                #[no_mangle]
                pub unsafe extern "C" fn [<ffi_ $name>](msg: *const ffi::c_char) {
                    let c_str = CStr::from_ptr(msg);
                    tracing::$name!("{}", c_str.to_string_lossy());
                }
            }
        )+
    };
}

declare_log_ffi!(error, warn, info, debug);
