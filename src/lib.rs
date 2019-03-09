extern crate proc_macro;
#[macro_use] extern crate quote;
use std::mem;
extern crate syn;

use syn::*;
use syn::synom::Synom;
use syn::punctuated::Punctuated;

use proc_macro::TokenStream;

#[derive(Debug)]
struct Args {
    name: Ident,
    extra: Vec<Lit>,
}

impl Synom for Args {
    named!(parse -> Self,
        do_parse!(
            name: syn!(Ident) >>
            extra: option!(
                map!(parens!(Punctuated::<Lit, Token![,]>::parse_terminated_nonempty),
                        |(_parens, vars)| vars.into_iter().collect())
            ) >> 
            (Args {
                name,
                extra: extra.unwrap_or(vec!())
            })
        )
    );
}

#[proc_macro_attribute]
pub fn adorn(arg: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: Args = match parse(arg) {
        Ok(arg) => arg,
        Err(..) => panic!("#[adorn()] takes a single identifier input, followed by optional literal parameters in parentheses"),
    };
    let mut input: ItemFn = match parse(input) {
        Ok(input) => input,
        Err(..) => panic!("#[adorn()] must be applied on functions"),
    };

    if input.decl.generics.where_clause.is_some() {
        panic!("#[adorn()] does not work with where clauses")
    }

    let id: Ident = "_decorated_fn".into();
    let old_ident =  mem::replace(&mut input.ident, id);
    let mut i = 0;
    let mut exprs = Vec::with_capacity(input.decl.inputs.len()+1);
    exprs.push(quote!(#id));
    for extra_arg in macro_args.extra {
        exprs.push(quote!(#extra_arg));
    }
    let mut args = vec!();
    for arg in input.decl.inputs.iter() {
        let arg_ident: Ident = format!("_arg_{}", i).into();

        match *arg {
            FnArg::Captured(ref cap) => {
                let ty = &cap.ty;
                args.push(quote!(#arg_ident: #ty));
            }
             _ => panic!("Unexpected argument {:?}", arg)
        }
        exprs.push(quote!(#arg_ident));
        i += 1;
    }


    let decorator = &macro_args.name;
    let attributes = &input.attrs;
    let vis = &input.vis;
    let constness = &input.constness;
    let unsafety = &input.unsafety;
    let abi = &input.abi;
    let generics = &input.decl.generics;
    let output = &input.decl.output;
    let outer = quote!(
        #(#attributes),*
        #vis #constness #unsafety #abi fn #old_ident #generics(#(#args),*) #output {
            #input
            #decorator(#(#exprs),*)
        }
    );

    outer.into()
}

#[proc_macro_attribute]
pub fn make_decorator(arg: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: Args = match parse(arg) {
        Ok(arg) => arg,
        _ => panic!("#[make_decorator()] takes a single identifier input"),
    };
    if !macro_args.extra.is_empty() {
        panic!("#[make_decorator()] takes a single identifier input");
    }

    let input: ItemFn = match parse(input) {
        Ok(input) => input,
        Err(..) => panic!("#[make_decorator()] must be applied on functions"),
    };

    if input.decl.generics.where_clause.is_some() {
        panic!("#[make_decorator()] does not work with where clauses")
    }

    let mut args = vec![];

    let caller_name = &macro_args.name;
    args.push(quote!(#caller_name: _F));
    let mut where_args = vec![];
    let mut pat_args = vec![];
    for arg in input.decl.inputs.iter() {
        match *arg {
            FnArg::Captured(ref cap) => {
                let ty = &cap.ty;
                let pat = &cap.pat;
                where_args.push(quote!(#ty));
                pat_args.push(quote!(#pat));
                args.push(quote!(#pat: #ty));
            }
             _ => panic!("Unexpected argument {:?}", arg)
        }
    }

    let funcname = &input.ident;
    let attributes = &input.attrs;
    let vis = &input.vis;
    let constness = &input.constness;
    let unsafety = &input.unsafety;
    let abi = &input.abi;
    let output = &input.decl.output;
    let body = &input.block;

    quote!(
        #(#attributes),*
        #vis #constness #unsafety #abi fn #funcname <_F> (#(#args),*) #output where _F: (Fn(#(#where_args),*) #output) {
            #body
        }
    ).into()
}
