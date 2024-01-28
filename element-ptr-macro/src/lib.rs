extern crate proc_macro;

use proc_macro2::{Ident, Span, TokenStream};

use proc_macro_crate::FoundCrate;
use quote::{quote, ToTokens};
use syn::{
    bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, token, Expr, Index, LitInt, Token, Type,
};

mod quote_into_hack;
use quote_into_hack::quote_into;

#[proc_macro]
pub fn element_ptr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as MacroInput);

    let base_crate = {
        let found = proc_macro_crate::crate_name("element-ptr").unwrap_or(FoundCrate::Itself);

        match found {
            FoundCrate::Itself => String::from("element_ptr"),
            FoundCrate::Name(name) => name,
        }
    };

    let base_crate = Ident::new(&base_crate, Span::call_site());

    let ctx = AccessListToTokensCtx {
        list: &input.body,
        base_crate: &base_crate,
    };

    let ptr = input.ptr;

    (quote! {
        {
            let ptr = #ptr;
            :: #base_crate ::helper::element_ptr_unsafe();
            #[allow(unused_unsafe)]
            unsafe {
                let ptr = :: #base_crate ::helper::new_pointer(ptr);
                #ctx
            }
        }
    })
    .into()
}

struct AccessList(Vec<ElementAccess>);

struct AccessListToTokensCtx<'i> {
    list: &'i AccessList,
    base_crate: &'i Ident,
}

impl<'i> ToTokens for AccessListToTokensCtx<'i> {
    fn to_tokens(&self, mut tokens: &mut TokenStream) {
        let base_crate = self.base_crate;

        let mut dirty = false;

        for access in &self.list.0 {
            use ElementAccess::*;

            if dirty {
                quote_into! { tokens =>
                    let ptr = :: #base_crate ::helper::new_pointer(ptr);
                };
                dirty = false;
            }

            match access {
                Field(FieldAccess { _dot, field }) => match &field {
                    Some(FieldAccessType::Named(ident)) => quote_into! { tokens =>
                        let ptr = ptr.copy_addr(
                            ::core::ptr::addr_of!( ( *ptr.into_const() ) . #ident )
                        );
                    },
                    Some(FieldAccessType::Tuple(index)) => quote_into! { tokens =>
                        let ptr = ptr.copy_addr(
                            ::core::ptr::addr_of!( ( *ptr.into_const() ) . #index )
                        );
                    },
                    Some(FieldAccessType::Deref(..)) => {
                        dirty = true;
                        quote_into! { tokens =>
                            let ptr = ptr.read();
                        }
                    }
                    // output something for r-a autocomplete.
                    None => {
                        // honestly i'm not quite sure why this specifically
                        // lets r-a autocomplete after the dot, but it does, and also
                        // gives a correct (and sort of fake) compiler error of
                        // "unexpected token `)`".
                        // i wish there was a better way to interact with r-a about this,
                        // but this hack will have to do.
                        let error = syn::Error::new_spanned(
                            _dot,
                            "expected an identifier, integer literal, or `*` after this `.`",
                        )
                        .into_compile_error();
                        quote_into! { tokens =>
                            let ptr = ptr.copy_addr(
                                ::core::ptr::addr_of!( ( *ptr.into_const() ) #_dot )
                            );
                            #error;
                        }
                        // just stop generating from here.
                        return;
                    }
                },
                Index(IndexAccess { index, .. }) => quote_into! { tokens =>
                    let ptr = :: #base_crate ::helper::index(ptr, #index);
                },
                Offset(access) => {
                    let name = match (&access.offset_type, access.byte.is_some()) {
                        (OffsetType::Add(..), false) => Ident::new("add", Span::call_site()),
                        (OffsetType::Sub(..), false) => Ident::new("sub", Span::call_site()),
                        (OffsetType::Add(..), true) => Ident::new("byte_add", Span::call_site()),
                        (OffsetType::Sub(..), true) => Ident::new("byte_sub", Span::call_site()),
                    };
                    let offset = &access.value;
                    quote_into! { tokens =>
                        let ptr = ptr . #name ( #offset );
                    }
                }
                Cast(CastAccess { ty, .. }) => quote_into! { tokens =>
                    let ptr = ptr.cast::<#ty>();
                },
                Group(access) => {
                    let list = AccessListToTokensCtx {
                        list: &access.inner,
                        base_crate: self.base_crate,
                    };
                    quote_into! { tokens =>
                        let ptr = {
                            #list
                        };
                    };
                    dirty = true;
                }
            };
        }
        if dirty {
            quote_into! { tokens =>
                ptr
            };
        } else {
            quote_into! { tokens =>
                ptr.into_inner()
            };
        }
    }
}

impl Parse for AccessList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut out = Vec::new();
        while !input.is_empty() {
            let access: ElementAccess = input.parse()?;
            if access.is_final() && !input.is_empty() {
                return Err(input.error(""));
            }
            out.push(access);
        }
        Ok(Self(out))
    }
}

struct MacroInput {
    ptr: Expr,
    _arrow: Token![=>],
    body: AccessList,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ptr: input.parse()?,
            _arrow: input.parse()?,
            body: input.parse()?,
        })
    }
}

enum ElementAccess {
    Field(FieldAccess),
    Index(IndexAccess),
    Offset(OffsetAccess),
    Cast(CastAccess),
    Group(GroupAccess),
}

impl ElementAccess {
    fn is_final(&self) -> bool {
        match self {
            Self::Cast(acc) => acc.arrow.is_none(),
            _ => false,
        }
    }
}

impl Parse for ElementAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![.]) {
            input.parse().map(Self::Field)
        } else if input.peek(token::Bracket) {
            input.parse().map(Self::Index)
        } else if input.peek(kw::u8) || input.peek(Token![+]) || input.peek(Token![-]) {
            input.parse().map(Self::Offset)
        } else if input.peek(Token![as]) {
            input.parse().map(Self::Cast)
        } else if input.peek(token::Paren) {
            input.parse().map(Self::Group)
        } else {
            Err(input.error("expected valid element access"))
        }
    }
}

// Also includes deref because it is similar.
struct FieldAccess {
    _dot: Token![.],
    field: Option<FieldAccessType>,
}

impl Parse for FieldAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _dot: input.parse()?,
            field: {
                if input.is_empty() {
                    None
                } else {
                    Some(input.parse()?)
                }
            },
        })
    }
}

enum FieldAccessType {
    Named(Ident),
    Tuple(Index),
    Deref(Token![*]),
}

impl Parse for FieldAccessType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let l = input.lookahead1();
        if l.peek(Token![*]) {
            input.parse().map(Self::Deref)
        } else if l.peek(syn::Ident) {
            input.parse().map(Self::Named)
        } else if l.peek(LitInt) {
            // no amazing way to do this unfortunately.
            input.parse().map(Self::Tuple)
        } else {
            Err(l.error())
        }
    }
}

struct IndexAccess {
    _bracket: token::Bracket,
    index: Expr,
}

impl Parse for IndexAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _bracket: bracketed!(content in input),
            index: content.parse()?,
        })
    }
}

// struct DerefAccess {
//     dot: Token![.],
//     star: Token![*],
// }

// impl Parse for DerefAccess {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         Ok(Self {
//             dot: input.parse()?,
//             star: input.parse()?,
//         })
//     }
// }

struct OffsetAccess {
    byte: Option<kw::u8>,
    offset_type: OffsetType,
    value: OffsetValue,
}

impl Parse for OffsetAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            byte: input.parse()?,
            offset_type: input.parse()?,
            value: input.parse()?,
        })
    }
}

enum OffsetType {
    Add(Token![+]),
    Sub(Token![-]),
}

impl Parse for OffsetType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let l = input.lookahead1();
        if l.peek(Token![+]) {
            input.parse().map(Self::Add)
        } else if l.peek(Token![-]) {
            input.parse().map(Self::Sub)
        } else {
            Err(l.error())
        }
    }
}

enum OffsetValue {
    Integer { int: LitInt },
    Grouped { _paren: token::Paren, expr: Expr },
}

impl Parse for OffsetValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let l = input.lookahead1();
        if l.peek(token::Paren) {
            let content;
            Ok(Self::Grouped {
                _paren: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else if l.peek(LitInt) {
            Ok(Self::Integer {
                int: input.parse()?,
            })
        } else {
            Err(l.error())
        }
    }
}

impl ToTokens for OffsetValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Integer { int } => int.to_tokens(tokens),
            Self::Grouped { expr, .. } => expr.to_tokens(tokens),
        }
    }
}

struct CastAccess {
    _as_token: Token![as],
    ty: Type,
    // TODO: is this best syntax for this?
    arrow: Option<Token![=>]>,
}

impl Parse for CastAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _as_token: input.parse()?,
            ty: input.parse()?,
            arrow: input.parse()?,
        })
    }
}

struct GroupAccess {
    _paren: token::Paren,
    inner: AccessList,
}

impl Parse for GroupAccess {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _paren: parenthesized!(content in input),
            inner: content.parse()?,
        })
    }
}

mod kw {
    syn::custom_keyword!(u8);
}
