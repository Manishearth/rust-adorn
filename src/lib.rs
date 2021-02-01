extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate quote;
use std::mem;
extern crate syn;
extern crate core;

use syn::*;
use syn::synom::Synom;
use syn::punctuated::{Punctuated, Pair};

use proc_macro::{TokenStream};
use proc_macro2::{Span, TokenStream as TokenStream2, TokenNode, TokenTree};
use std::str::FromStr;
use quote::{ToTokens};
use std::iter::FromIterator;
use std::ops::Index;

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
pub fn adorn_static(arg: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: Args = match parse(arg) {
        Ok(arg) => arg,
        Err(..) => panic!("#[adorn_static()] takes a single identifier input, followed by optional literal parameters in parentheses"),
    };
    let input: ImplItemMethod = match parse(input) {
        Ok(input) => input,
        Err(..) => panic!("#[adorn_static()] must be applied on methods"),
    };

    if input.sig.decl.generics.where_clause.is_some() {
        panic!("#[adorn_static()] does not work with where clauses")
    }

    match input.sig.decl.inputs.first() {
        Some(Pair::End(FnArg::SelfValue(..)))
        | Some(Pair::End(FnArg::SelfRef(..)))
        | Some(Pair::Punctuated(FnArg::SelfRef(..), _))
        | Some(Pair::Punctuated(FnArg::SelfValue(..), _)) =>
            panic!("#[make_decorator_static()] must be applied on static methods"),
        _ => {}
    }

    let id: Ident = "_decorated_fn".into();
    let old_ident =  input.sig.ident.clone();
    let mut i = 0;
    let mut exprs = Vec::with_capacity(input.sig.decl.inputs.len()+1);
    exprs.push(quote!(#id));
    for extra_arg in macro_args.extra {
        exprs.push(quote!(#extra_arg));
    }
    let mut args = vec!();
    for arg in input.sig.decl.inputs.iter() {
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

    let closure = ExprClosure {
        attrs: input.attrs.clone(),
        capture: None,
        or1_token: Token![|]([Span::call_site()]),
        inputs: input.sig.decl.inputs.clone(),
        or2_token: Token![|]([Span::call_site()]),
        output: input.sig.decl.output.clone(),
        body: Box::new(Expr::Block(ExprBlock{attrs: vec![], block: input.block.clone()}))
    };

    let decorator = &macro_args.name;
    let attributes = &input.attrs;
    let vis = &input.vis;
    let constness = &input.sig.constness;
    let unsafety = &input.sig.unsafety;
    let abi = &input.sig.abi;
    let generics = &input.sig.decl.generics;
    let output = &input.sig.decl.output;
    let defaultness = &input.defaultness;

    let outer = quote!(
        #(#attributes),*
        #vis #defaultness #constness #unsafety #abi fn #old_ident #generics(#(#args),*) #output {
            let #id = #closure;
            Self::#decorator(#(#exprs),*)
        }
    );

    outer.into()
}

#[proc_macro_attribute]
pub fn adorn_method(arg: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: Args = match parse(arg) {
        Ok(arg) => arg,
        Err(..) => panic!("#[adorn_static()] takes a single identifier input, followed by optional literal parameters in parentheses"),
    };
    let input_backup = TokenStream2::from(input.clone());
    let input_method: ImplItemMethod = match parse(input) {
        Ok(input) => input,
        Err(..) => panic!("#[adorn_static()] must be applied on methods"),
    };

    if input_method.sig.decl.generics.where_clause.is_some() {
        panic!("#[adorn_static()] does not work with where clauses")
    }

    fn resolve_name(tokens: TokenStream2, conflict: &mut bool, candidate: &mut String) {
        *conflict = true;
        let mut tokens = tokens;
        while *conflict {
            *conflict = false;
            let tokens_backup = tokens.clone();
            for token in tokens {
                match token.kind {
                    TokenNode::Group(_, inner) => {
                        resolve_name(inner, conflict, candidate)
                    },
                    TokenNode::Term(ref term) => {
                        let id_str = term.as_str();
                        if *candidate == id_str {
                            *conflict = true;
                            *candidate = "_".to_owned() + candidate;
                        }
                    }
                    _ => {}
                }
            }
            tokens = tokens_backup;
        }
    }

    let mut new_self = "_self".to_owned();
    resolve_name(input_backup, &mut true, &mut new_self);

    match input_method.sig.decl.inputs.first() {
        Some(Pair::End(FnArg::SelfValue(..)))
        | Some(Pair::End(FnArg::SelfRef(..)))
        | Some(Pair::Punctuated(FnArg::SelfValue(..), _))
        | Some(Pair::Punctuated(FnArg::SelfRef(..), _)) => {},
        _ => panic!("#[adorn_method()] must be applied on nonstatic methods")
    };

    let id: Ident = "_decorated_fn".into();
    let old_ident =  input_method.sig.ident.clone();
    let mut exprs = Vec::with_capacity(input_method.sig.decl.inputs.len()+1);
    exprs.push(quote!(#id));
    for extra_arg in macro_args.extra {
        exprs.push(quote!(#extra_arg));
    }
    let mut args = vec!();
    args.push(input_method.sig.decl.inputs.index(0).into_tokens());
    for i in 1..input_method.sig.decl.inputs.len() {
        let arg = &input_method.sig.decl.inputs[i];
        let arg_ident: Ident = format!("_arg_{}", i).into();

        match *arg {
            FnArg::Captured(ref cap) => {
                let ty = &cap.ty;
                args.push(quote!(#arg_ident: #ty));
            }
            _ => panic!("Unexpected argument {:?}", arg)
        }
        exprs.push(quote!(#arg_ident));
    }


    fn replace_arg(arg: &mut FnArg, new_self: &str) {
        let (pat, ty) = match arg {
            FnArg::SelfValue(ArgSelf{mutability, ..}) => {
                let tmp_ty = TokenStream2::from_str("Self");
                debug_assert!(tmp_ty.is_ok());
                (
                    Pat::from(
                        PatIdent{
                            by_ref: None,
                            mutability: mutability.clone(),
                            ident: Ident::from(new_self),
                            subpat: None
                        }
                    ),
                    Type::from(TypeVerbatim {tts: tmp_ty.unwrap()})
                )
            }
            FnArg::SelfRef(
                ArgSelfRef{
                    and_token,
                    mutability,
                    lifetime,
                    ..
                }
            ) => {
                let tmp_ty = TokenStream2::from_str("Self");
                debug_assert!(tmp_ty.is_ok());
                (
                    Pat::from(
                        PatIdent{
                            by_ref: None,
                            mutability: None,
                            ident: Ident::from(new_self),
                            subpat: None
                        }
                    ),
                    Type::from(
                        TypeReference{
                            and_token: and_token.clone(),
                            lifetime: lifetime.clone(),
                            mutability: (*mutability).clone(),
                            elem: Box::new(Type::from(TypeVerbatim {tts: tmp_ty.unwrap()}))
                        }
                    )
                )
            }
            _ => return
        };

        let new_arg =
            FnArg::Captured(
                ArgCaptured{
                    pat,
                    ty,
                    colon_token: Token![:]([Span::call_site()])
                }
            );
        let _ = mem::replace(arg, new_arg);
    }

    fn get_new_tokenstream(tts: TokenStream2, new_self: &str) -> TokenStream2 {
        fn get_new_tokentree(tt: TokenTree, new_self: &str) -> TokenTree {
            fn get_new_tokennode(tn: TokenNode, new_self: &str) -> TokenNode {
                match tn {
                    TokenNode::Group(d, ts) =>
                        TokenNode::Group(d, get_new_tokenstream(ts, new_self)),
                    TokenNode::Term(ti) => {
                        if ti.as_str() == "self" {
                            TokenNode::Term(proc_macro2::Term::intern(new_self))
                        }
                        else {
                            TokenNode::Term(ti)
                        }
                    },
                    other @ _ => other
                }
            }
            TokenTree {
                span: tt.span.clone(),
                kind: get_new_tokennode(tt.kind, new_self)
            }
        }

        let mut retval = Vec::new();
        for t in tts {
            retval.push(get_new_tokentree(t, new_self));
        }
        TokenStream2::from_iter(retval)
    }

    fn get_new_block(block: Block, new_self: &str, fn_id: &str) -> Block {
        let ts = TokenStream2::from_iter(block.into_tokens());
        let new_ts = get_new_tokenstream(ts, new_self);
        let result: Result<Block, syn::synom::ParseError> = parse2(new_ts);
        if result.is_ok() {
            result.unwrap()
        } else {
            panic!("Error occurs after replacing `self' in method {} by {}", fn_id, new_self)
        }
    }

    let mut new_args = input_method.sig.decl.inputs.clone();
    replace_arg(&mut new_args[0], &new_self);

    let new_block =
        get_new_block(input_method.block.clone(), &new_self,&old_ident.to_string());

    let closure = ExprClosure {
        attrs: input_method.attrs.clone(),
        capture: None,
        or1_token: Token![|]([Span::call_site()]),
        inputs: new_args,
        or2_token: Token![|]([Span::call_site()]),
        output: input_method.sig.decl.output.clone(),
        body: Box::new(Expr::Block(ExprBlock{attrs: vec![], block: new_block}))
    };

    let decorator = &macro_args.name;
    let attributes = &input_method.attrs;
    let vis = &input_method.vis;
    let constness = &input_method.sig.constness;
    let unsafety = &input_method.sig.unsafety;
    let abi = &input_method.sig.abi;
    let generics = &input_method.sig.decl.generics;
    let output = &input_method.sig.decl.output;
    let defaultness = &input_method.defaultness;

    let outer = quote!(
        #(#attributes),*
        #vis #defaultness #constness #unsafety #abi fn #old_ident #generics(#(#args),*) #output {
            let #id = #closure;
            self.#decorator(#(#exprs),*)
        }
    );

    outer.into()
}

#[proc_macro_attribute]
pub fn make_decorator_static(arg: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: Args = match parse(arg) {
        Ok(arg) => arg,
        _ => panic!("#[make_decorator_static()] takes a single identifier input"),
    };
    if !macro_args.extra.is_empty() {
        panic!("#[make_decorator_static()] takes a single identifier input");
    }

    let input: ImplItemMethod = match parse(input) {
        Ok(input) => input,
        Err(..) => panic!("#[make_decorator_static()] must be applied on methods"),
    };

    if input.sig.decl.generics.where_clause.is_some() {
        panic!("#[make_decorator_static()] does not work with where clauses")
    }

    match input.sig.decl.inputs.first() {
        | Some(Pair::End(FnArg::SelfValue(..)))
        | Some(Pair::End(FnArg::SelfRef(..)))
        | Some(Pair::Punctuated(FnArg::SelfRef(..), _))
        | Some(Pair::Punctuated(FnArg::SelfValue(..), _)) =>
            panic!("#[make_decorator_static()] must be applied on static methods"),
        _ => {}
    }

    let mut args = vec![];

    let caller_name = &macro_args.name;
    args.push(quote!(#caller_name: _F));
    let mut where_args = vec![];
    for arg in input.sig.decl.inputs.iter() {
        match *arg {
            FnArg::Captured(ref cap) => {
                let ty = &cap.ty;
                let pat = &cap.pat;
                where_args.push(quote!(#ty));
                args.push(quote!(#pat: #ty));
            }
            _ => panic!("Unexpected argument {:?}", arg)
        }
    }

    let funcname = &input.sig.ident;
    let attributes = &input.attrs;
    let vis = &input.vis;
    let constness = &input.sig.constness;
    let unsafety = &input.sig.unsafety;
    let abi = &input.sig.abi;
    let output = &input.sig.decl.output;
    let body = &input.block;
    let defaultness = &input.defaultness;

    quote!(
        #(#attributes),*
        #vis #defaultness #constness #unsafety #abi fn #funcname <_F> (#(#args),*) #output where _F: (Fn(#(#where_args),*) #output) {
            #body
        }
    ).into()
}

#[proc_macro_attribute]
pub fn make_decorator_method(arg: TokenStream, input: TokenStream) -> TokenStream {
    let macro_args: Args = match parse(arg) {
        Ok(arg) => arg,
        _ => panic!("#[make_decorator_static()] takes a single identifier input"),
    };
    if !macro_args.extra.is_empty() {
        panic!("#[make_decorator_static()] takes a single identifier input");
    }

    let input: ImplItemMethod = match parse(input) {
        Ok(input) => input,
        Err(..) => panic!("#[make_decorator_static()] must be applied on methods"),
    };

    if input.sig.decl.generics.where_clause.is_some() {
        panic!("#[make_decorator_static()] does not work with where clauses")
    }

    let first_arg = match input.sig.decl.inputs.first() {
        Some(Pair::End(first_arg @ FnArg::SelfValue(..)))
        | Some(Pair::End(first_arg @ FnArg::SelfRef(..)))
        | Some(Pair::Punctuated(first_arg @ FnArg::SelfValue(..), _))
        | Some(Pair::Punctuated(first_arg @ FnArg::SelfRef(..), _)) =>
            first_arg,
        _ => panic!("#[make_decorator_method()] must be applied on nonstatic methods")
    };

    let caller_name = &macro_args.name;
    let mut args = vec![];
    let mut where_args = vec![];

    match first_arg {
        FnArg::SelfValue(ref arg_self) => {
            let tmp = TokenStream2::from_str("Self");
            debug_assert!(tmp.is_ok());
            let ty = Type::from(TypeVerbatim {tts: tmp.unwrap()});
            where_args.push(ty.into_tokens());
            args.push(quote!(#arg_self));
        },
        FnArg::SelfRef(
            ref arg_self_ref @ ArgSelfRef { .. }
        ) => {
            let tmp = TokenStream2::from_str("Self");
            debug_assert!(tmp.is_ok());
            let ty = Type::from(
                TypeReference{
                    and_token: arg_self_ref.and_token.clone(),
                    lifetime: arg_self_ref.lifetime.clone(),
                    mutability: arg_self_ref.mutability.clone(),
                    elem: Box::new(Type::from(TypeVerbatim {tts: tmp.unwrap()}))
                }
            );
            where_args.push(quote!(#ty));
            args.push(quote!(#arg_self_ref));
        }
        _ => panic!()
    }
    args.push(quote!(#caller_name: _F));

    for i in 1..input.sig.decl.inputs.len() {
        let arg = &input.sig.decl.inputs[i];
        match *arg {
            FnArg::Captured(ref cap) => {
                let ty = &cap.ty;
                let pat = &cap.pat;
                where_args.push(quote!(#ty));
                args.push(quote!(#pat: #ty));
            }
            _ => panic!("Unexpected argument {:?}", arg)
        }
    }

    let funcname = &input.sig.ident;
    let attributes = &input.attrs;
    let vis = &input.vis;
    let constness = &input.sig.constness;
    let unsafety = &input.sig.unsafety;
    let abi = &input.sig.abi;
    let output = &input.sig.decl.output;
    let body = &input.block;
    let defaultness = &input.defaultness;

    quote!(
        #(#attributes),*
        #vis #defaultness #constness #unsafety #abi fn #funcname <_F> (#(#args),*) #output where _F: (Fn(#(#where_args),*) #output) {
            #body
        }
    ).into()
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
    for arg in input.decl.inputs.iter() {
        match *arg {
            FnArg::Captured(ref cap) => {
                let ty = &cap.ty;
                let pat = &cap.pat;
                where_args.push(quote!(#ty));
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
