extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;
use syn::DeriveInput;

/// # MachineImpl
///
/// This crate provides a custom derive for `MachineImpl`.
/// `MachineImpl` tags an enum as an instruction set and simplifies
/// the building of Machines that implement the instruction set.
///
/// # Examples
///
/// ```rust
/// use machine_impl::*;
///
/// #[derive(Clone, MachineImpl)]
/// pub enum Example {
/// Red, Green, Yellow,
/// }
/// ```
/// This will create two new alias, a Sender and Receiver:
/// ```ignore, rust
/// pub type ExampleSender = ::smol::channel::Sender<Example>;
/// pub type ExampleReceiver = ::smol::channel::Receiver<Example>;
/// ```
/// Additionally, it will implement the MachineImpl trait for Example:
/// ```ignore, rust
/// impl MachineImpl for Example {
///     type Adapter = Example;
///     type InstructionSet = Example;
/// }
/// ```
/// Finally, it will provide an implementation for the MachineBuilder trait:
/// ```ignore, rust
/// impl MachineBuilder for Example {
///     type InstructionSet = Example;          
/// }
/// ```
/// This all leads to building a machine that implements the instruction set.
/// ```
/// # use machine_impl::*;
/// use server_core::*;
/// use machine_foundation::*;
///
/// # #[derive(Clone, MachineImpl)]
/// # pub enum Example {
/// # Red, Green, Yellow,
/// # }
///
/// struct Alice {}
/// impl Machine<Example> for Alice {
///     fn receive(&self, cmd: Example, sender: &mut MachineSender) {}
/// }
/// let (alice, sender) = machine::create(Alice {});
/// ::smol::block_on(async {sender.send(Example::Red).await.ok()});
/// ```
#[proc_macro_derive(MachineImpl)]
pub fn derive_machine_impl_fn(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let sender_ident = format_ident!("{}Sender", name);
    let receiver_ident = format_ident!("{}Receiver", name);
    let expanded = quote! {
        #[automatically_derived]
        #[allow(unused_qualifications)]
        pub type #sender_ident = ::smol::channel::Sender<#name>;
        pub type #receiver_ident = ::smol::channel::Receiver<#name>;
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl server_core::MachineImpl for #name {
            type Adapter = #name;
            type InstructionSet = #name;
        }

        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl server_core::MachineBuilder for #name {
            type InstructionSet = #name;
        }
    };
    TokenStream::from(expanded)
}
