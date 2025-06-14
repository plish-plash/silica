use proc_macro2::{Punct, Spacing, TokenStream};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Expr, ExprClosure, FieldValue, Ident, Token,
};

struct StructValues {
    fields: Punctuated<FieldValue, Token![,]>,
    rest: Option<Expr>,
}

impl Parse for StructValues {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut fields = Punctuated::new();
        let mut rest = None;
        loop {
            if input.is_empty() {
                break;
            }
            if input.peek(Token![..]) {
                input.parse::<Token![..]>()?;
                rest = Some(input.parse::<Expr>()?);
                break;
            }
            let value = FieldValue::parse(input)?;
            fields.push_value(value);
            if input.is_empty() {
                break;
            }
            let punct = input.parse()?;
            fields.push_punct(punct);
        }
        Ok(StructValues { fields, rest })
    }
}
impl ToTokens for StructValues {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_terminated(self.fields.iter(), Punct::new(',', Spacing::Alone));
        if let Some(rest) = self.rest.as_ref() {
            tokens.append_all(quote_spanned! {rest.span()=> ..#rest });
        } else {
            tokens.append_all(quote_spanned! {self.fields.span()=> ..Default::default() });
        }
    }
}

enum WidgetOrExpr {
    Widget(Widget),
    Expr(Expr),
}

impl Parse for WidgetOrExpr {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek2(token::Paren) {
            input.parse().map(Self::Widget)
        } else {
            input.parse().map(Self::Expr)
        }
    }
}
impl ToTokens for WidgetOrExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all(quote! { let child = });
        match self {
            WidgetOrExpr::Widget(widget) => widget.to_tokens(tokens),
            WidgetOrExpr::Expr(expr) => expr.to_tokens(tokens),
        }
        tokens.append_all(quote! { ; gui.add_child(widget, child); });
    }
}

struct Widget {
    name: Ident,
    properties: StructValues,
    event: Option<ExprClosure>,
    children: Punctuated<WidgetOrExpr, Token![,]>,
}

impl Parse for Widget {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let properties;
        parenthesized!(properties in input);
        let properties = properties.parse()?;
        let event = if input.peek(Token![|]) {
            Some(input.parse()?)
        } else {
            None
        };
        let children = if input.peek(token::Brace) {
            let children;
            braced!(children in input);
            children.parse_terminated(WidgetOrExpr::parse, Token![,])?
        } else {
            Punctuated::new()
        };
        Ok(Widget {
            name,
            properties,
            event,
            children,
        })
    }
}
impl ToTokens for Widget {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        token::Brace::default().surround(tokens, |tokens| {
            let name = &self.name;
            let properties = &self.properties;
            let event = self
                .event
                .as_ref()
                .map(|event| quote_spanned! {event.span()=> , move #event });
            tokens.append_all(quote_spanned! {name.span()=>
                type Properties<'a> = <#name as WidgetBuilder>::Properties<'a>;
                let widget = <#name>::create(gui, Properties { #properties } #event);
            });
            tokens.append_all(self.children.iter());
            tokens.append(Ident::new("widget", proc_macro2::Span::call_site()));
        });
    }
}

/// Creates a `taffy::Style`. `taffy` must be in-scope. Arguments to the macro become the fields of
/// a struct literal, with `..Default::default()` added if not present. For example:
/// ```layout!(flex_direction: FlexDirection::Row, padding: Rect::length(8.0))```
/// becomes
/// ```taffy::Style { flex_direction: taffy::FlexDirection::Row, padding: taffy::Rect::length(8.0), ..Default::default() }```
/// The taffy prelude is in-scope inside of the macro, so none of the taffy types need to be
/// imported.
#[proc_macro]
pub fn layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let values = parse_macro_input!(input as StructValues);
    quote! {{ use taffy::prelude::*; Style { #values } }}.into()
}

/// Creates a gui node, optionally with children. The new node will be unparented and so should
/// usually be given a parent with `gui.add_child` or made the root with `gui.set_root`. In order to
/// use this macro, `WidgetBuilder` must be in-scope, as well as the widgets used. Additionally,
/// there must be a `gui` variable in-scope with the type `&mut Gui`.
///
/// This macro expands widget definitions, which look like `Widget(fields)`, into calls like
/// `Widget::create(gui, WidgetProperties { fields })`. Similar to the `layout!` macro, the
/// arguments inside the parentheses become the fields of a struct literal, with
/// `..Default::default()` added if not present. The `WidgetProperties` type comes from the widget's
/// `WidgetBuilder` implementation. You can use your own widgets with this macro by implementing
/// `WidgetBuilder` and defining a `create` function.
///
/// Here is an example that creates a label:
/// ```let label = gui! { Label(text: "Hello!", layout: layout!(flex_grow: 1.0)) };```
/// 
/// Children can optionally be listed after the widget in braces, such as `Widget(fields) { child1, child2 }`. Children can be either
/// widget definitions or expressions (such as variables). Here is an example that creates a container with two labels:
/// ```let my_label = gui! { Label(text: username) };
/// let container = gui! {
///     Container(layout: layout!(flex_direction: FlexDirection::Column)) {
///         Label(text: "Hello,"),
///         my_label,
///     }
/// };```
///
/// Finally, a closure can be specified after the widget (before children), and will be passed as a
/// third argument to `create` if present. `move` will be added to this closure automatically. If
/// the macro gives an error about "expected 3 arguments, found 2", you have forgotten a closure on
/// one of your widgets. Here is an example that creates a button: ```let button = gui! {
///     Button(label: Some("Click Here!")) |_gui: &mut Gui| {
///         println!("button clicked");
///     }
/// };```
#[proc_macro]
pub fn gui(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let widget = parse_macro_input!(input as Widget);
    widget.to_token_stream().into()
}
