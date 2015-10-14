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
use syntax::owned_slice::OwnedSlice;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("adorn"), SyntaxExtension::MultiModifier(box adorn));
    reg.register_syntax_extension(intern("make_decorator"), SyntaxExtension::MultiModifier(box make_decorator));
}

fn adorn(cx: &mut ExtCtxt, sp: Span, mitem: &MetaItem, item: Annotatable) -> Annotatable {
    let (funcname, dec_args) = {
        let err = || cx.span_err(sp, r##"#[adorn] should be of the format `#[adorn(foo)]` or
                                         `#[adorn(foo(a = "arg1", a = "arg2"))], where `foo` is the decorator method"##);
        if let MetaList(_, ref l) = mitem.node {
            if l.len() == 1 {
                match l[0].node {
                    MetaWord(ref is) => (Ident::with_empty_ctxt(intern(&*is)), vec![]),
                    MetaList(ref is, ref list) => {
                        let mut errored = false;
                        let strs = list.iter().filter_map(|i| {
                            if let MetaNameValue(_, ref l) = i.node {
                                Some(l.node.clone())
                            } else {
                                errored = true;
                                None
                            }
                        }).collect::<Vec<_>>();
                        if errored {
                            err();
                            return item;
                        }
                        (Ident::with_empty_ctxt(intern(&*is)), strs)
                    }
                    _ => {
                        err();
                        return item;
                    }
                }
            } else {
                err();
                return item;         
            }
        } else {
            err();
            return item;
        }
    };
    match item {
        Annotatable::Item(ref it) => {
            if let ItemFn(ref decl, unsafety, constness, abi, ref generics, _) = it.node {
                let id = Ident::with_empty_ctxt(intern("_decorated_fn"));
                let maindecl = decl.clone();
                let mut i = 0;
                let mut exprs = Vec::with_capacity(decl.inputs.len()+1);
                exprs.push(cx.expr_path(cx.path_ident(sp, id)));
                for s in dec_args {
                    exprs.push(cx.expr_lit(sp, s));
                }
                let maindecl = maindecl.map(|mut m| {
                    for ref mut arg in m.inputs.iter_mut() {
                        let arg_ident = Ident::with_empty_ctxt(intern(&format!("_arg_{}", i)[..]));
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
                let newfn = ItemFn(maindecl, unsafety, constness, abi, generics.clone(),
                                   cx.block(sp, vec![inner], Some(call)));
                Annotatable::Item(cx.item(sp, ident, attrs, newfn))
            } else {
                cx.span_err(sp, "#[adorn] only allowed on functions");
                return item.clone()
            }
        }
        _ => {
            cx.span_err(sp, "#[adorn] only allowed on functions");
            item
        }
    }
}

fn make_decorator(cx: &mut ExtCtxt, sp: Span, mitem: &MetaItem, item: Annotatable) -> Annotatable {
    let funcname = if let MetaList(_, ref l) = mitem.node {
        if l.len() == 1 {
            if let MetaWord(ref is) = l[0].node {
               Ident::with_empty_ctxt(intern(&*is))
            } else {
                cx.span_err(sp, "#[make_decorator] should be of the format #[make_decorator(f)], where `f`\
                                 is the identifier for the extra function argument created");
                return item;
            }
        } else {
            cx.span_err(sp, "#[make_decorator] should be of the format #[make_decorator(f)], where `f`\
                             is the identifier for the extra function argument created");
            return item;         
        }
    } else {
            cx.span_err(sp, "#[make_decorator] should be of the format #[make_decorator(f)], where `f`\
                             is the identifier for the extra function argument created");
        return item;
    };
    match item {
        Annotatable::Item(ref it) => {
            if let ItemFn(ref decl, unsafety, constness, abi, ref generics, ref blk) = it.node {
                let ty_ident = Ident::with_empty_ctxt(intern("_F"));
                let ty = cx.ty_ident(sp, ty_ident);
                let output = if let Return(ref t) = decl.output {
                    Some(t.clone())
                } else {
                    None
                };
                let paramdata = ParenthesizedParameterData {
                    span: sp,
                    inputs: decl.inputs.iter().map(|ref x| x.ty.clone()).collect(),
                    output: output.clone(),
                };
                let path = Path {
                    span: sp,
                    global: false,
                    segments: vec![PathSegment {
                        identifier: Ident::with_empty_ctxt(intern("Fn")),
                        parameters: ParenthesizedParameters(paramdata)
                    }],
                };
                let typaram = cx.typaram(sp, ty_ident, OwnedSlice::from_vec(vec![cx.typarambound(path)]), None);
                let mut bounds = generics.ty_params.clone().into_vec();
                bounds.push(typaram);
                let gen = Generics {
                    lifetimes: generics.lifetimes.clone(),
                    ty_params: OwnedSlice::from_vec(bounds),
                    where_clause: generics.where_clause.clone(),
                };
                let mut inputs = decl.inputs.clone();
                inputs.insert(0, cx.arg(sp, funcname, ty));
                let decl = cx.fn_decl(inputs, output.unwrap_or(cx.ty(sp, TyTup(Vec::new()))));
                let func = ItemFn(decl, unsafety, constness, abi, gen, blk.clone());
                Annotatable::Item(cx.item(sp, it.ident, it.attrs.clone(), func))
            } else {
                cx.span_err(sp, "#[make_decorator] only allowed on functions");
                return item.clone()
            }
        }
        _ => {
            cx.span_err(sp, "#[make_decorator] only allowed on functions");
            item
        }
    }
}