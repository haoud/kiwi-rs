use proc_macro::TokenStream;
use syn::{ItemFn, ItemStatic, StaticMutability, parse_macro_input};

/// An attribute macro that places the annotated function in the `.init.text`
/// section. After the kernel initialization, the `.init.text` section will
/// be discarded, freeing up the memory used by the initialization code.
///
/// # Safety
/// The function annotated with this attribute must not be called after the
/// initialization of the kernel, since the code will be discarded after
/// initialization.
#[proc_macro_attribute]
pub fn init(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);

    if input_fn.sig.asyncness.is_some() {
        panic!("The #[init] attribute cannot be applied to async functions");
    }

    if input_fn.sig.unsafety.is_none() {
        panic!("The #[init] attribute cannot be applied to safe functions");
    }

    let section = syn::parse_quote!(#[unsafe(link_section = ".init.text")]);
    input_fn.attrs.push(section);
    TokenStream::from(quote::quote!(#input_fn))
}

/// An attribute macro that places the annotated static variable in the
/// `.init.data` section. After the kernel initialization, the `.init.data`
/// section will be discarded, freeing up the memory used by the
/// initialization data.
///
/// # Safety
/// The static variable annotated with this attribute must not be accessed
/// after the initialization of the kernel, since the data will be discarded
/// after initialization and potentially reused for other purposes.
#[proc_macro_attribute]
pub fn initdata(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut var = parse_macro_input!(item as ItemStatic);

    if let StaticMutability::None = var.mutability {
        panic!("The #[initdata] attribute cannot be applied to immutable static variables");
    }

    let section = syn::parse_quote!(#[unsafe(link_section = ".init.data")]);
    var.attrs.push(section);
    TokenStream::from(quote::quote!(#var))
}

#[proc_macro_attribute]
pub fn per_cpu(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut var = parse_macro_input!(item as ItemStatic);

    // Create some identifiers based on the name of the variable
    let name = var.ident.clone();
    let name_ctor_fn = syn::Ident::new(&format!("__init_percpu_{}", name), name.span());
    let name_storage = syn::Ident::new(&format!("__{}_STORAGE", name), name.span());
    let name_ctor = syn::Ident::new(&format!("__{}_CTOR", name), name.span());

    // We will replace the original variable with a new one that is of type
    // `PerCpu<T>` where `T` is the original type of the variable. We also
    // modify the initializer of the variable to initialize it using a
    // constructor function that will be called for each CPU during the
    // initialization of the percpu area.
    let old_type = var.ty.clone();
    let old_init = var.expr.clone();
    let new_type = syn::parse_quote!(crate::arch::percpu::PerCpu<#old_type>);
    let new_init = syn::parse_quote!(
        unsafe {
            crate::arch::percpu::PerCpu::new(&raw const #name_storage)
        }
    );

    *var.expr = new_init;
    *var.ty = new_type;

    TokenStream::from(quote::quote!(
        /// Reserve some space in the percpu section for the variable. We will
        /// not use this directly, but it will be used to calculate the offset
        /// of the variable within the percpu section and by extension, within
        /// the percpu area of each CPU.
        #[unsafe(link_section = ".data.percpu")]
        static #name_storage: crate::arch::percpu::PerCpuStorage<#old_type>
            = unsafe { crate::arch::percpu::PerCpuStorage::new() };

        /// Register a constructor function that will be called for each CPU
        /// during initialization. This will be discarded after initialization
        /// since no CPU core can be added or removed after initialization of
        /// the kernel.
        ///
        /// We must make sure that the constructor function will not be removed
        /// by the linker, since it is not called directly by any code by
        /// adding the `#[used]` attribute
        #[used]
        #[unsafe(link_section = ".percpu.ctors")]
        static mut #name_ctor: fn() = #name_ctor_fn;

        /// The constructor function that will be called for each CPU during
        /// initialization. This function will calculate the offset of the
        /// variable within the percpu section and initialize it for each CPU
        /// using the expression that was originally used to initialize the
        /// variable.
        /// TODO: Support non-const initializer ? This should be easy and this
        /// may offer some optimization opportunities.
        #[unsafe(link_section = ".init.text")]
        #[allow(non_snake_case)]
        fn #name_ctor_fn() {
            let offset = #name_storage.percpu_offset();
            let ptr = crate::arch::percpu::from_offset::<#old_type>(offset);
            unsafe { ptr.cast_mut().write(#old_init) };
        }

        #var
    ))
}
