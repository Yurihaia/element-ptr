#![allow(unused)]

// ripped from quote. uses private API. lol. lmao.
// idk why quote doesn't have this. would make it so much more efficient.
#[cfg(feature = "quote_into_hack")]
macro_rules! quote_into {
    // Special case rule for a single tt, for performance.
    ($stream:expr => $tt:tt) => {{
        let mut _s = &mut $stream;
        quote::quote_token! { $tt _s };
    }};

    // Special case rules for two tts, for performance.
    ($stream:expr => # $var:ident) => {{
        quote::ToTokens::to_tokens(&$var, &mut $stream);
    }};
    ($stream:expr => $tt1:tt $tt2:tt) => {{
        let mut _s = &mut $stream;
        quote::quote_token! { $tt1 _s };
        quote::quote_token! { $tt2 _s };
    }};

    // Rule for any other number of tokens.
    ($stream:expr => $($tt:tt)*) => {{
        let mut _s = &mut $stream;
        quote::quote_each_token!{ _s $($tt)* };
    }};
}

#[cfg(not(feature = "quote_into_hack"))]
macro_rules! quote_into {
    ($stream:expr => $($t:tt)*) => { {
        (&mut $stream).extend(quote::quote! { $($t)* });
    } };
}

#[cfg(feature = "quote_into_hack")]
macro_rules! quote_spanned_into {
    // Special case rule for a single tt, for performance.
    ($stream:expr, $span:expr => $tt:tt) => {{
        let mut _s = &mut $stream;
        let _span: quote::__private::Span = quote::__private::get_span($span).__into_span();
        quote::quote_token_spanned! { $tt _s _span };
        _s
    }};

    // Special case rules for two tts, for performance.
    ($stream:expr, $span:expr=> # $var:ident) => {{
        let _: quote::__private::Span = quote::__private::get_span($span).__into_span();
        quote::ToTokens::to_tokens(&$var, &mut $stream);
        _s
    }};
    ($stream:expr, $span:expr=> $tt1:tt $tt2:tt) => {{
        let mut _s = &mut $stream;
        let _span: quote::__private::Span = quote::__private::get_span($span).__into_span();
        quote::quote_token_spanned! { $tt1 _s _span };
        quote::quote_token_spanned! { $tt2 _s _span };
        _s
    }};

    // Rule for any other number of tokens.
    ($stream:expr, $span:expr=> $($tt:tt)*) => {{
        let mut _s = &mut $stream;
        let _span: quote::__private::Span = quote::__private::get_span($span).__into_span();
        quote::quote_each_token_spanned! { _s _span $($tt)* };
        _s
    }};
}

#[cfg(not(feature = "quote_into_hack"))]
macro_rules! quote_spanned_into {
    ($stream:expr, $span:expr => $($t:tt)*) => { {
        (&mut $stream).extend(quote::quote_spanned! { $span=> $($t)* });
    } };
}

pub(crate) use {quote_into, quote_spanned_into};