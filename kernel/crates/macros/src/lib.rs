use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

/// A macro to indicate that a function is only used during the initialization
/// of the kernel. This macro will this attribute are put in a separate .init
/// section. When the kernel has been initialized, this section will be
/// discarded and the memory will be freed, allowing the kernel to reduce its
/// memory footprint and enhance cache locality.
///
/// # Safety
/// If an function with this attribute is called after the kernel has been
/// initialized, the behavior is undefined and will probably cause a kernel
/// panic.
#[proc_macro_attribute]
pub fn init(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);
    let link_section = syn::parse_quote!(#[link_section = ".init"]);

    input_fn.sig.unsafety = Some(syn::parse_quote!(unsafe));
    input_fn.attrs.push(link_section);

    TokenStream::from(quote::quote!(
        #input_fn
    ))
}
