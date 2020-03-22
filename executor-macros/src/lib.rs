#[allow(unused_extern_crates)]
extern crate proc_macro;
extern crate alloc;
use proc_macro::{Delimiter, TokenStream, TokenTree};
use alloc::str::FromStr;

#[proc_macro_attribute]
pub fn entry(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = item.into_iter().peekable();
    if let Some(TokenTree::Ident(t)) = input.next() {
        if t.to_string() != "async" {
            panic!("Expected \"async\"")
        }
    } else {
        panic!("Expected \"async\"")
    }
    if let Some(TokenTree::Ident(t)) = input.next() {
        if t.to_string() != "fn" {
            panic!("Expected \"fn\"")
        }
    } else {
        panic!("Expected \"fn\"")
    }
    let function_name = if let Some(TokenTree::Ident(t)) = input.next() {
        t.to_string()
    } else {
        panic!("Expected function name")
    };
    let func_params = if let Some(TokenTree::Group(t)) = input.next() {
        if t.delimiter() == Delimiter::Parenthesis {
            t.stream().to_string()
        } else {
            panic!("Expected function paramters")
        }
    } else {
        panic!("Expected function paramters")
    };
    let function_content = if let Some(TokenTree::Group(t)) = input.next() {
        if t.delimiter() == Delimiter::Brace {
            t.stream().to_string()
        } else {
            panic!("Expected function content")
        }
    } else {
        panic!("Expected function content")
    };
    let func = &format!(
        "fn {}({}) {{
        executor::spawn(async move{{ 
            {}   
       }})
    }}",
        function_name, func_params, function_content
    );
    TokenStream::from_str(func).unwrap()
}

#[proc_macro_attribute]
pub fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = item.into_iter().peekable();
    if let Some(TokenTree::Ident(t)) = input.next() {
        if t.to_string() != "async" {
            panic!("Expected \"async\"")
        }
    } else {
        panic!("Expected \"async\"")
    }
    if let Some(TokenTree::Ident(t)) = input.next() {
        if t.to_string() != "fn" {
            panic!("Expected \"fn\"")
        }
    } else {
        panic!("Expected \"fn\"")
    }
    if let Some(TokenTree::Ident(t)) = input.next() {
        let s = t.to_string();
        if s != "main" {
            panic!("Expected \"main\"")
        }
    } else {
        panic!("Expected function name")
    }
    let func_params = if let Some(TokenTree::Group(t)) = input.next() {
        if t.delimiter() == Delimiter::Parenthesis {
            t.stream().to_string()
        } else {
            panic!("Expected function paramters")
        }
    } else {
        panic!("Expected function paramters")
    };
    let function_content = if let Some(TokenTree::Group(t)) = input.next() {
        if t.delimiter() == Delimiter::Brace {
            t.stream().to_string()
        } else {
            panic!("Expected function content")
        }
    } else {
        panic!("Expected function content")
    };
    let func = &format!(
        "fn main({}) {{
        let complete = std::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
        let ender = complete.clone();
        std::thread::spawn(||{{
            executor::spawn(async move {{
                (async {{
                    {}
                }}).await;
                ender.store(true, core::sync::atomic::Ordering::Release);
            }});
        }});
        while !complete.load(core::sync::atomic::Ordering::Acquire) {{}}
    }}",
        func_params, function_content
    );
    TokenStream::from_str(func).unwrap()
}
