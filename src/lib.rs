#![feature(plugin_registrar, box_syntax)]

#![feature(rustc_private)]

#[macro_use]
extern crate syntax;
#[macro_use]
extern crate rustc;

use rustc::plugin::Registry;
use  syntax::ext::base::SyntaxExtension;

use syntax::ast::*;
use syntax::codemap::Span;
use syntax::ext::base::{ExtCtxt, Annotatable};
use syntax::ext::build::AstBuilder;
use syntax::parse::token::intern;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("adorn"), SyntaxExtension::MultiModifier(box adorn));
}

fn adorn(cx: &mut ExtCtxt, sp: Span, mitem: &MetaItem, item: Annotatable) -> Annotatable {
    let funcname = if let MetaList(_, ref l) = mitem.node {
        if l.len() == 1 {
            if let MetaWord(ref is) = l[0].node {
               intern(&*is).ident() 
            } else {
                cx.span_err(sp, "#[adorn] should be of the format #[adorn(foo)], where `foo` is the decorator method");
                return item;
            }
        } else {
            cx.span_err(sp, "#[adorn] should be of the format #[adorn(foo)], where `foo` is the decorator method");
            return item;         
        }
    } else {
        cx.span_err(sp, "#[adorn] should be of the format #[adorn(foo)], where `foo` is the decorator method");
        return item;
    };
    match item {
        Annotatable::Item(ref it) => {
            if let ItemFn(ref decl, unsafety, abi, ref generics, _) = it.node {
                let id = intern("_decorated_fn").ident();
                let maindecl = decl.clone();
                let mut i = 0;
                let mut exprs = Vec::with_capacity(decl.inputs.len()+1);
                exprs.push(cx.expr_path(cx.path_ident(sp, id)));
                let maindecl = maindecl.map(|mut m| {
                    for ref mut arg in m.inputs.iter_mut() {
                        let arg_ident = intern(&format!("_arg_{}", i)[..]).ident();
                        arg.pat = cx.pat_ident(sp, arg_ident);
                        exprs.push(cx.expr_ident(sp, arg_ident));
                        i += 1;
                    }
                    m
                });

                let call = cx.expr_call_ident(sp, funcname, exprs);
                let (ident, attrs) = (it.ident.clone(), it.attrs.clone());
                let innerfn = it.clone();
                let innerfn = innerfn.map(|mut inf| { inf.ident = id; inf });
                let inner = cx.stmt_item(sp, innerfn);
                let newfn = ItemFn(maindecl, unsafety, abi, generics.clone(), cx.block(sp, vec![inner], Some(call)));
                Annotatable::Item(cx.item(sp, ident, attrs, newfn))
            } else {
                return item.clone()
            }
        }
        _ => item
    }
}