use proc_macro::TokenStream;
use syn::{ItemFn, ItemStatic, parse_macro_input};

/// A macro to indicate that a function is only used during the initialization
/// of the kernel. This macro will this attribute are put in a separate .init
/// section. When the kernel has been initialized, this section will be
/// discarded and the memory will be freed, allowing the kernel to reduce its
/// memory footprint and enhance cache locality.
///
/// # Panics
/// This macro panics if it is applied to a non-unsafe function. The caller must
/// ensure that the function will not be called after the kernel has been
/// initialized. Calling such a function after initialization will lead to undefined
/// behavior, since its code and data may have been discarded.
///
/// # Safety
/// If an function with this attribute is called after the kernel has been
/// initialized, the behavior is undefined and will probably cause a kernel
/// panic.
#[proc_macro_attribute]
pub fn init(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let link_section = syn::parse_quote!(#[unsafe(link_section = ".init")]);

    if input_fn.sig.unsafety.is_none() {
        panic!("The `init` attribute can only be applied to unsafe functions");
    }

    input_fn.attrs.push(link_section);
    TokenStream::from(quote::quote!(
        #input_fn
    ))
}

/// A macro to indicate that a static variable is only used during the
/// initialization of the kernel. This macro will this attribute are put in a
/// separate .init section. When the kernel has been initialized, this section
/// will be discarded and the memory will be freed, allowing the kernel to
/// reduce its memory footprint and enhance cache locality.
///
/// # Safety
/// The caller must ensure that the static variable will not be accessed after
/// the kernel has been initialized. Accessing such a variable after
/// initialization will lead to undefined behavior, since its data may have
/// been discarded. HOWEVER, the compiler will not enforce this rule, and
/// this is impossible to mark a static variable as unsafe to access. Thus,
/// the responsibility is entirely on the developer to ensure that the static
/// variable is not accessed after initialization.
#[proc_macro_attribute]
pub fn initdata(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_static = parse_macro_input!(item as ItemStatic);
    let link_section = syn::parse_quote!(#[unsafe(link_section = ".init.data")]);

    input_static.attrs.push(link_section);
    TokenStream::from(quote::quote!(
        #input_static
    ))
}
