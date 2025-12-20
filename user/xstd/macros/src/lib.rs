use proc_macro::TokenStream;
use syn::{ItemFn, parse_macro_input};

/// A macro to indicate that a function is the main entry point of a user
/// application. This macro will create the necessary boilerplate to set up the
/// user application environment before calling the main function.
#[proc_macro_attribute]
pub fn main(_: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let input_fn_name = &input_fn.sig.ident;

    // Verify that the function name is not `_start` to avoid multiple
    // definitions of the entry point.
    if input_fn_name == "_start" {
        return TokenStream::from(quote::quote!(
            compile_error!("The function name `_start` is reserved for the \
            entry point. Please use a different name for your main function.");
        ));
    }

    // TODO: Depending on the return type of the input function, we might want to
    // handle it differently (e.g., if it returns a Result, we might want to
    // exit with a non-zero code on error, or if it returns !, we might not need to
    // call exit at all). For now, we assume it returns ().
    TokenStream::from(quote::quote!(
        #input_fn

        #[unsafe(no_mangle)]
        pub unsafe fn _start() -> ! {
            #input_fn_name();
            xstd::task::exit(0);
        }
    ))
}
