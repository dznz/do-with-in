extern crate proc_macro;
extern crate syn;
#[macro_use] extern crate quote;
extern crate proc_macro2;

use proc_macro::{TokenStream, TokenTree};
use proc_macro2::TokenTree as TokenTree2;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::ToTokens;
use syn::{parse, Attribute, PathSegment, Result, Token};
use syn::parse::{Parse, ParseStream, Parser, Peek};
use syn::spanned::Spanned;
use syn::{Expr, Ident, Type, Visibility};
use syn::punctuated::Punctuated;
use syn::parenthesized;
use syn::token::Token;
use syn::buffer::Cursor;

use std::marker::PhantomData;

use std::collections::HashMap;
use std::fmt::format;

#[derive(Debug,Copy,Clone,PartialEq,Eq)]
enum Sigil {
  Dollar,
  Percent,
  Hash,
}

impl Default for Sigil {
  fn default() -> Self {
    Sigil::Dollar
  }
}

#[derive(Debug,Clone)]
struct Configuration<Start: StartMarker> where Start: Clone {
  allow_prelude: bool,
  sigil: Sigil,
  rest: Option<TokenStream2>,
  _do: PhantomData<Start>,
}

type PeekFn = fn(Cursor) -> bool;

trait StartMarker {
  fn name() -> Option<String>;
  //fn type() -> Self::token;
  type token: Parse;// = syn::token::Do;
  fn tokenp() -> PeekFn;// = syn::token::Do;
  type tokend: Parse + ToString + Clone;
}

impl StartMarker for DoMarker {
  fn name() -> Option<String> {
    None //Some(String::from("do"))
  }
  //fn type() -> Self::token {
  //  return (Token![do])
  //}
  type token = syn::token::Do;
  fn tokenp() -> PeekFn {
    syn::token::Do::peek
  }
  type tokend = syn::Ident;
}

#[derive(Debug,Clone)]
struct DoMarker;

impl<T: StartMarker + Clone> Default for Configuration<T> {
  fn default() -> Self {
    //dbg!("Configuration<T>::default()");
    Configuration { allow_prelude: true, sigil: Sigil::default(), rest: None, _do: PhantomData }
  }
}

struct Fatuous {
  fat: TokenStream,
}

impl Parse for Fatuous {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut fat = TokenStream2::new();
    input.step(|cursor| {
      let mut rest = *cursor;
      while let Some((tt, next)) = rest.token_tree() {
        fat.extend(TokenStream2::from(tt).into_iter());
        rest = next;
      }
      Ok(((), rest))
    });
    Ok(Fatuous { fat: fat.into() })
  }
}


impl<T: StartMarker + Clone> Parse for Configuration<T> {
  fn parse(input: ParseStream) -> Result<Self> {
    //dbg!("Start of parsing configuration.");
    let mut base_config: Configuration<T> = Default::default();
    //dbg!("Made base config.");
    while !input.is_empty() {
      //dbg!("Start of while.");
      let mut next: Option<&str> = None;
      let mut foo: String = String::from("");
      if let Some(name) = T::name() {
        if let Ok(it) = input.parse::<T::tokend>() {
          if it.to_string().as_str() == name {
            break;
          }
          foo = it.to_string().clone();
          next = Some(foo.as_str().clone());
        }
      } else if T::tokenp()(input.cursor()) {
          //dbg!("iwhflwhedflowhedfl");
          if let Ok(it) = input.parse::<T::token>() {
            break;
          }
      }
      let mut st: String = String::from("");
      let err_pos = input.fork();
      let new_next = if let Some(it) = next { it } else if !input.is_empty() { st = input.parse::<Ident>().expect("blergh").to_string(); &st } else { break; };
      match new_next {
        "sigil" => {
          //dbg!("sigil found");
          input.parse::<Token![:]>()?;
          if input.peek(Token![$]) {
            input.parse::<Token![$]>()?;
            base_config.sigil = Sigil::Dollar;
          } else if input.peek(Token![%]) {
            input.parse::<Token![%]>()?;
            base_config.sigil = Sigil::Percent;
          } else if input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            base_config.sigil = Sigil::Hash;
          }
        },
        a => {return Err(err_pos.error(format!("Bad configuration section; found {} when sigil or end of prelude expected", a)));},
      };
    }
    let mut fat = TokenStream2::new();
    input.step(|cursor| {
      let mut rest = *cursor;
      while let Some((tt, next)) = rest.token_tree() {
        fat.extend(TokenStream2::from(tt).into_iter());
        rest = next;
      }
      Ok(((), rest))
    });

    base_config.rest = Some(fat.into());
    //while(input.parse
    //dbg!("End of parsing configuration.");
    Ok(base_config)
  }
}

impl<T: StartMarker + Clone> Configuration<T> {
  fn name(&self) -> Option<String> {
    T::name()
  }
}

#[derive(Clone)]
struct Variables<'a, T: StartMarker + Clone> {
  handlers:    Handlers<'a, T>,
  with_interp: HashMap<String, TokenStream2>,
  no_interp:   HashMap<String, TokenStream2>,
}

impl<'a, T: 'static + StartMarker + Clone> Default for Variables<'a, T> {
  fn default() -> Self {
    Variables { handlers: genericDefaultHandlers::<'a, T>(), with_interp: HashMap::new(), no_interp: HashMap::new() }
  }
}

type Handler<T: StartMarker + Clone> = dyn Fn(Configuration<T>, Variables<T>, TokenStream2) -> (Variables<T>, TokenStream2);
type Handlers<'a, T: StartMarker + Clone> = HashMap<String, Box<&'a Handler<T>>>;


fn ifHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  (v, quote!{println!("todo");}.into())
}

fn concatHandlerInner<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> syn::parse::Result<String> {
  let mut accumulator: Vec<String> = Vec::new();
  for token in t.into_iter() {
    if let TokenTree2::Literal(lit) = token.clone() {
      let real_lit = syn::parse_str::<syn::Lit>(&lit.clone().to_string());
      match real_lit {
        Ok(syn::Lit::Str(it)) => accumulator.push(it.value()),
        Ok(x)            => accumulator.push(lit.to_string()),
        Err(err)         => return Err(err),
      }
      //accumulator.push(lit.to_string());
    } else if let TokenTree2::Group(grp) = token.clone() {
      // Recurse into groups
      match concatHandlerInner(c.clone(), v.clone(), grp.stream()) {
        Ok(it)   => accumulator.push(it),
        Err(err) => return Err(err),
      }
    } else {
      let msg = format!("Expected a literal (literal string, number, character or etc), got {}.", token);
      return Err(syn::parse::Error::new_spanned(token, msg));
    }
  }
  let out_str: String = accumulator.into_iter().collect();
  return Ok(out_str);
}

fn concatHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  let mut output = TokenStream2::new();
  let mut variables = v.clone();
  let mut stream = t.into_iter();
  let concat_token = stream.next();
  if let Some(TokenTree2::Ident(name)) = concat_token.clone() {
    let mut temp = TokenStream2::new();
    temp.extend(stream);
    let new_token_stream = do_with_in_explicit(temp, c.clone(), v.clone());
    match concatHandlerInner(c.clone(), v.clone(), new_token_stream) {
      Ok(it)   => output.extend(TokenStream2::from(TokenTree2::Literal(proc_macro2::Literal::string(&it)))),
      Err(err) => return (v, err.to_compile_error()),
    }
  } else if let Some(it) = concat_token {
    let msg = format!("Expected 'concat' to absolutely start a concat expression, got {}.", it);
    return (v, quote!{compile_error!{ #msg }}.into());
  }
  return (v, output);
}

fn string_to_identHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  let mut output = TokenStream2::new();
  let mut variables = v.clone();
  let mut stream = t.into_iter();
  let string_to_ident_token = stream.next();
  if let Some(TokenTree2::Ident(name)) = string_to_ident_token.clone() {
    let mut temp = TokenStream2::new();
    temp.extend(stream);
    let mut new_token_stream_iter = do_with_in_explicit(temp, c.clone(), v.clone()).into_iter();
    match new_token_stream_iter.next() {
      Some(TokenTree2::Literal(lit)) => {
        let real_lit = syn::parse_str::<syn::Lit>(&lit.clone().to_string());
        match real_lit {
          Ok(syn::Lit::Str(it)) => output.extend(TokenStream2::from(TokenTree2::Ident(proc_macro2::Ident::new(&it.value(), lit.span())))),
          Ok(x)            => return (v, quote!{compiler_error!{ "Expected a string." }}.into()),
          Err(err)         => return (v, err.to_compile_error()),
        }
      },
      Some(x) => {
        let msg = format!("Expected a literal, got {}.", x);
        return (v, quote!{compile_error!{ #msg }}.into());
      },
      None    => return (v, quote!{compile_error!{ "No string given; cannot create identifier." }}.into()),
    }
  } else if let Some(it) = string_to_ident_token {
    let msg = format!("Expected 'string_to_ident' to absolutely start a string_to_ident expression, got {}.", it);
    return (v, quote!{compile_error!{ #msg }}.into());
  }
  return (v, output);
}

fn forHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  let mut output = TokenStream2::new();
  let mut variables = v.clone();
  let mut stream = t.into_iter();
  let for_token = stream.next();
  if let Some(TokenTree2::Ident(name)) = for_token.clone() {
    for token in stream {
      
    }
  } else if let Some(it) = for_token {
    let msg = format!("Expected 'for' to absolutely start a for expression, got {}.", it);
    return (v, quote!{compile_error!{ #msg }}.into());
  } else {
    return (v, quote!{compile_error!{ "For expression stream was unexpectedly empty." }}.into());
  }
  (v, output)
}

enum Operator {
  Plus,
  Times,
  Minus,
  Division,
}

fn arithmeticInternal<T: StartMarker + Clone, N: std::str::FromStr + std::ops::Add<Output=N> + std::ops::Div<Output=N> + std::ops::Mul<Output=N> + std::ops::Sub<Output=N>>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> syn::parse::Result<N> where <N as std::str::FromStr>::Err: std::fmt::Display {
  let mut left: Option<N> = None;
  let mut operator: Option<Operator> = None;
  for token in t.clone().into_iter() {
    match left {
      None => {
        left = match token.clone() {
          TokenTree2::Literal(lit) => Some(syn::parse_str::<syn::LitInt>(&lit.to_string())?.base10_parse::<N>()?),
          TokenTree2::Group(grp) => Some(arithmeticInternal::<T, N>(c.clone(), v.clone(), grp.stream())?),
          it => {
            let msg = format!("Expected number, got {}", it);
            return Err(syn::parse::Error::new_spanned(token, msg));
          },
        }
      },
      Some(num) => {
        match operator {
          None => {
            match token.clone() {
              TokenTree2::Punct(punct) => {
                match punct.as_char() {
                  '+' if punct.spacing() == proc_macro2::Spacing::Alone => {
                    operator = Some(Operator::Plus);
                  },
                  '-' if punct.spacing() == proc_macro2::Spacing::Alone => {
                    operator = Some(Operator::Minus);
                  },
                  '*' if punct.spacing() == proc_macro2::Spacing::Alone => {
                    operator = Some(Operator::Times);
                  },
                  '/' if punct.spacing() == proc_macro2::Spacing::Alone => {
                    operator = Some(Operator::Division);
                  },
                  it   => {
                    let msg = format!("Expected operator such as +, *, -, or /, got {}", it);
                    return Err(syn::parse::Error::new_spanned(token, msg));
                  },
                }
                left = Some(num);
              },
              it => {
                let msg = format!("Expected operator such as +, *, -, or /, got {}", it);
                return Err(syn::parse::Error::new_spanned(token, msg));
              },
            }
          },
          Some(op) => {
            let right = match token.clone() {
              TokenTree2::Literal(lit) => syn::parse_str::<syn::LitInt>(&lit.to_string())?.base10_parse::<N>()?,
              TokenTree2::Group(grp) => arithmeticInternal::<T, N>(c.clone(), v.clone(), grp.stream())?,
              it => {
                let msg = format!("Expected number, got {}", it);
                return Err(syn::parse::Error::new_spanned(token, msg));
              },
            };
            left = Some(match op {
              Operator::Plus     => num + right,
              Operator::Times    => num * right,
              Operator::Minus    => num - right,
              Operator::Division => num / right,
            }); //replace with: left = Some(result) 
            operator = None;
          },
        }
      },
    }
  }
  return match left {
    Some(n) => Ok(n),
    None    => Err(syn::parse::Error::new_spanned(t, "No numbers found.")),
  };
}

fn arithmeticHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  let mut output = TokenStream2::new();
  let mut variables = v.clone();
  let mut stream = t.into_iter();
  let ar_token = stream.next();
  if let Some(TokenTree2::Ident(name)) = ar_token.clone() {
    let mut temp = TokenStream2::new();
    temp.extend(stream);
    let new_token_stream = do_with_in_explicit(temp, c.clone(), v.clone());
    let mut new_token_stream_iter = new_token_stream.into_iter();
    match new_token_stream_iter.next() {
      Some(TokenTree2::Ident(var_token)) => {
        let mut temp2 = TokenStream2::new();
        temp2.extend(new_token_stream_iter);
       //variables.with_interp.insert(var_token.to_string(), 
        match var_token.to_string().as_str() {
          "u64" => {
            let out = proc_macro2::Literal::u64_suffixed(match arithmeticInternal::<T, u64>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "u32" => {
            let out = proc_macro2::Literal::u32_suffixed(match arithmeticInternal::<T, u32>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "u16" => {
            let out = proc_macro2::Literal::u16_suffixed(match arithmeticInternal::<T, u16>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "u8" => {
            let out = proc_macro2::Literal::u8_suffixed(match arithmeticInternal::<T, u8>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
          "i64" => {
            let out = proc_macro2::Literal::i64_suffixed(match arithmeticInternal::<T, i64>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "i32" => {
            let out = proc_macro2::Literal::i32_suffixed(match arithmeticInternal::<T, i32>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "i16" => {
            let out = proc_macro2::Literal::i16_suffixed(match arithmeticInternal::<T, i16>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "i8" => {
            let out = proc_macro2::Literal::i8_suffixed(match arithmeticInternal::<T, i8>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
          "f64" => {
            let out = proc_macro2::Literal::f64_suffixed(match arithmeticInternal::<T, f64>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
         "f32" => {
            let out = proc_macro2::Literal::f32_suffixed(match arithmeticInternal::<T, f32>(c.clone(), v.clone(), temp2) {
              Ok(x) => x,
              Err(err) => return (v, err.to_compile_error()),
            });
            output.extend(TokenStream2::from(TokenTree2::Literal(out)).into_iter());
          },
          it => {
            let msg = format!("Expected number type (u64, i64, f64, etc), got {}.", it);
            return (v, quote!{compile_error!{ #msg }}.into());
          }
        }
      },
      Some(x) => {},
      _ => {},
    }
  } else if let Some(it) = ar_token {
    let msg = format!("Expected 'arithmetic' first, got {}.", it);
    return (v, quote!{compile_error!{ #msg }}.into());
  } else {
    return (v, quote!{compile_error!{ "Arithmetic expression stream was unexpectedly empty." }}.into());
  }
  (v, output)
}



#[derive(Debug,Clone,PartialEq,Eq)]
enum LetState {
  LessThanNothing,
  Nothing,
  Name(String),
  NamePostEquals(String),
}

fn letHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  let mut variables = v.clone();
  let mut state: LetState = LetState::LessThanNothing;

  for token in t.into_iter() {
    match state.clone() {
      LetState::LessThanNothing => {
        // Consume the initial 'let'
        if let TokenTree2::Ident(name) = token.clone() {
          if name.to_string() == "let" {
            state = LetState::Nothing;
          } else {
            let msg = format!("Expected 'let' to absolutely start a let expression, got {}.", token);
            return (v, quote!{compile_error!{ #msg }}.into());
          }
        } else {
          let msg = format!("Expected 'let' to absolutely start a let expression, got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
        }
      },
      LetState::Nothing => {
        if let TokenTree2::Ident(name) = token {
          state = LetState::Name(name.to_string());
        } else {
          let msg = format!("Expected a variable name to start a let expression, got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
        }
      },
      LetState::Name(var_name) => {
        if let TokenTree2::Punct(punct) = token.clone() {
          if punct.as_char() == '=' && punct.spacing() == proc_macro2::Spacing::Alone {
            state = LetState::NamePostEquals(var_name);
          } else {
            let msg = format!("Expected '=', got {}.", token);
            return (v, quote!{compile_error!{ #msg }}.into());
          }
        } else {
          let msg = format!("Expected '=', got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
        }
      },
      LetState::NamePostEquals(var_name) => {
        if let TokenTree2::Group(body) = token {
          variables.no_interp.insert(var_name, body.stream());
          state = LetState::Nothing;
        } else {
          let msg = format!("Expected a curly bracket surrounded expression (the value to put in the variable), got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
       }
      },
    }
  }
  (variables, quote!{}.into())
}

fn varHandler<T: StartMarker + Clone>(c: Configuration<T>, v: Variables<T>, t: TokenStream2) -> (Variables<T>, TokenStream2) {
  let mut variables = v.clone();
  let mut state: LetState = LetState::LessThanNothing;

  for token in t.into_iter() {
    match state.clone() {
      LetState::LessThanNothing => {
        // Consume the initial 'let'
        if let TokenTree2::Ident(name) = token.clone() {
          if name.to_string() == "var" {
            state = LetState::Nothing;
          } else {
            let msg = format!("Expected 'var' to absolutely start a let expression, got {}.", token);
            return (v, quote!{compile_error!{ #msg }}.into());
          }
        } else {
          let msg = format!("Expected 'var' to absolutely start a let expression, got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
        }
      },
      LetState::Nothing => {
        if let TokenTree2::Ident(name) = token {
          state = LetState::Name(name.to_string());
        } else {
          let msg = format!("Expected a variable name to start a var expression, got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
        }
      },
      LetState::Name(var_name) => {
        if let TokenTree2::Punct(punct) = token.clone() {
          if punct.as_char() == '=' && punct.spacing() == proc_macro2::Spacing::Alone {
            state = LetState::NamePostEquals(var_name);
          } else {
            let msg = format!("Expected '=', got {}.", token);
            return (v, quote!{compile_error!{ #msg }}.into());
          }
        } else {
          let msg = format!("Expected '=', got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
        }
      },
      LetState::NamePostEquals(var_name) => {
        if let TokenTree2::Group(body) = token {
          //let to_insert = do_with_in_explicit(body.stream(), c, variables);
          variables.with_interp.insert(var_name, body.stream());
          state = LetState::Nothing;
        } else {
          let msg = format!("Expected a curly bracket surrounded expression (the value to put in the variable), got {}.", token);
          return (v, quote!{compile_error!{ #msg }}.into());
       }
      },
    }
  }
  (variables, quote!{}.into())
}

fn defaultHandlers() -> Handlers<'static, DoMarker> {
  let mut m: HashMap<String, Box<&Handler<DoMarker>>> = HashMap::new();
  m.insert(String::from("if"), Box::new(&ifHandler));
  m.insert(String::from("let"), Box::new(&letHandler));
  m.insert(String::from("var"), Box::new(&varHandler));
  m.insert(String::from("concat"), Box::new(&concatHandler));
  m.insert(String::from("string_to_ident"), Box::new(&string_to_identHandler));
  m.insert(String::from("arithmetic"), Box::new(&arithmeticHandler));
  m
}

fn genericDefaultHandlers<'a, T: 'static + StartMarker + Clone>() -> Handlers<'a, T> {
  let mut m: HashMap<String, Box<&Handler<T>>> = HashMap::new();
  m.insert(String::from("if"), Box::new(&ifHandler));
  m.insert(String::from("let"), Box::new(&letHandler));
  m.insert(String::from("var"), Box::new(&varHandler));
  m.insert(String::from("concat"), Box::new(&concatHandler));
  m.insert(String::from("string_to_ident"), Box::new(&string_to_identHandler));
  m.insert(String::from("arithmetic"), Box::new(&arithmeticHandler));
  m
}



#[proc_macro]
pub fn do_with_in(t: TokenStream) -> TokenStream {
  do_with_in_internal(t.into()).into()
}

fn do_with_in_internal(t: TokenStream2) -> TokenStream2 {
  // Check for configuration first
  match syn::parse2::<Configuration<DoMarker>>(t) {
    Ok(it) => {
      let mut configuration = it.clone();
      
      let out = match configuration.clone().rest {
        Some(out) => out,
        None      => TokenStream2::new().into(),
      };
      // For now to make testing possible
      configuration.rest = None;
      do_with_in_explicit(TokenStream2::from(out), configuration, Variables::default()).into()
    },
    Err(it) =>  it.to_compile_error().into()  // we actually want to early exit here, not do: do_with_in_explicit(it.to_compile_error().into(), Configuration::<DoMarker>::default(), defaultHandlers()),
  }
}


fn do_with_in_explicit<'a, T: StartMarker + Clone>(t: TokenStream2, c: Configuration<T>, v: Variables<'a, T>) -> TokenStream2 {
  let mut output = TokenStream2::new();
  let mut use_vars = v;
  //check for variables to insert
  //check for handlers to run
  //insert token
  let token_char = match c.clone().sigil {
    Sigil::Dollar  => '$',
    Sigil::Percent => '%',
    Sigil::Hash    => '#',
  };
  let mut expecting_variable = false;
  for token in t.into_iter() {
    match &token {
      TokenTree2::Punct(punct_char) if punct_char.spacing() == proc_macro2::Spacing::Alone && punct_char.as_char() == token_char => {
        if expecting_variable {
          expecting_variable = false;
          let out: TokenStream2 = TokenStream2::from(TokenTree2::Punct(punct_char.clone()));
          output.extend(out.into_iter());
        } else {
          expecting_variable = true;
        }
      },
      TokenTree2::Ident(ident) => {
        if expecting_variable {
          expecting_variable = false;
          let var_name = ident.to_string();
          // First we check for no interp, then interp
          if let Some(replace) = use_vars.no_interp.get(&var_name) {
            output.extend(replace.clone().into_iter());
          } else if let Some(replace) = use_vars.with_interp.get(&var_name) {
            output.extend(TokenStream2::from(do_with_in_explicit(replace.clone(), c.clone(), use_vars.clone())));
          }
        } else {
          output.extend(TokenStream2::from(TokenTree2::Ident(ident.clone())).into_iter());
        }
      },
      TokenTree2::Group(group) => {
        if expecting_variable {
          expecting_variable = false;
          // Check whether the handler matches
          let stream = group.stream();
          if !stream.is_empty() {
            let mut iter = stream.clone().into_iter();
            if let Some(TokenTree2::Ident(first)) = iter.next().clone() {
              if let Some(handler) = use_vars.clone().handlers.get(&first.to_string()) {
                let (new_vars, more_output) = handler(c.clone(), use_vars.clone(), stream);
                use_vars = new_vars;
                output.extend(more_output);
              }
            }

            //if let Some(handler) = v.handlers.get(
          }
        } else {
          output.extend(TokenStream2::from(TokenTree2::Group(group.clone())));
        }
      },
      a => {
        if expecting_variable {
          expecting_variable = false;
          let out: TokenStream2 = TokenStream2::from(TokenTree2::Punct(proc_macro2::Punct::new(token_char.clone(), proc_macro2::Spacing::Alone)));
          output.extend(out.into_iter());
        }
        output.extend(TokenStream2::from(a.clone()).into_iter());
      },
    }
  }
  output.into()
}

/*
fn with_maker(args: ArgList, body: Body) -> Handler {
  |c: Configuration, v: Variables, t: TokenStream| {
    // First match on the args
    // Then put substitutions in the body tokenstream
    (v, t)
  }
}

#[proc_macro_attribute]
fn do_with_in_izer(args: TokenStream, body: TokenStream) -> TokenStream {
  let mut configuration = defaultConfiguration();
  // Update configuration from args
  let new_body = quote!(
    let new_args = do_with_in_explicit(t);
    $body
  }
  new_body
}

*/

#[test]
fn conf_test_panic1() {
  let input: TokenStream2 = quote! {sigil: % ow2eihf do wiwlkef }.into();
  let output = do_with_in_internal(input);
  assert_eq!(format!("{}", output), format!("{}", TokenStream2::from(quote! {compile_error!{ "Bad configuration section; found ow2eihf when sigil or end of prelude expected" }} )));
}
