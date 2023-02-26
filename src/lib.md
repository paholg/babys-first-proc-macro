# Baby's First Proc Macro

I recently wrote my first proc-macro, and found it a bit hard to get started.
There's a lot of really basic documentation, some workshops, and very helpful
API documentation. But there's lacking in what I would call "intermediate"
tutorials, so that is what this will aim to be. You are expected to be
comfortable with Rust in general, but not with procedural macros at all, and we
will have a working, useful proc-macro at the end.

## Proc...macro?

Metaprogramming, or hacking on the language that you're using, is a very
powerful tool. It becames especially powerful when you get to hack on a language
using the language itself. That's what proc-macros, or prodecural macros, in
Rust let us do. They let us do things like custom `derive` statements, or sqlx's
ability to check your queries at compile-time by making database requests.

As input, you get some chunk of your program's code, and as output you get to
spit out some code that will later be compiled. Depending on the specific type
of proc-macro you're writing, you might even get to change the code that you're
given.

### The types

There are, as of this writing, three types of proc-macros.
* Function-like proc-macros, created with `#[proc_macro]`; these are called much
  like regular macros, with a `!`.
* Custom derives, created with `#[proc_macro_derive(DeriveName)]` and used with
  `#[derive(DeriveName)]`. Note that the input to these macros is the struct or
  enum definition they are attributed to, and it will be produced in your
  program without being included in your macro output.
* Attribute macros, created with `#[proc_macro_attribute]`, act somewhat like
  custom derives, but the input is not automatically included in your code; you
  will have to include it yourself if you want it.

For more information, see
[The Rust Reference](https://doc.rust-lang.org/reference/procedural-macros.html).

## Getting started

Let's get started by writing our own proc-macro! We're going to write a
simplified version of my first proc-macro,
[subenum](https://crates.io/crates/subenum). Subenum takes an enum as input, as well as
some attributes to designate which variants we're interested in, and will output
a new enum (or enums) with those variants, as well as the ability to convert and
compare between them.

That is, given

```ignore,rust
#[subenum(Dog)]
#[derive(Copy, Clone, Debug)]
enum Canis {
    Wolf,
    #[subenum(Dog)]
    GermanShephard,
    #[subenum(Dog)]
    Boxer,
    #[subenum(Dog)]
    GolderRetriever,
    Coyote,
}
```

we should produce

```ignore,rust
#[derive(Copy, Clone, Debug)]
enum Dog {
  GermanShephard,
  Boxer,
  GoldenRetriver,
}
```

with the ability to compare and convert between a `Dog` and a `Canis`.

## Baby's first steps

Let's get started! First thing we're going to use some crates that will do a lot
of heavy lifting for us. If we just use the standard library, we are stuck with
the [`TokenStream`](https://doc.rust-lang.org/proc_macro/struct.TokenStream.html)
type, which is not the easiest to work with.

Let's start with our `Cargo.toml`:

```toml
[package]
name = "babys-first-proc-macro"
version = "0.1.0"
edition = "2021"

[lib]
name = "subenum"
proc-macro = true

[dependencies]
quote = "1.0.23"
syn = "1.0.109"
proc-macro2 = "1.0.51"

[build-dependencies]
tango = "0.8.3"
```

The `lib.proc-macro = true` line is what tells Rust that we're writing a proc-
macro, and makes the `proc_macro` library available.

The `quote` crate gives us, among other things, the `quote!` macro which makes
it so we can produce code in similar ways we would when writing a normal macro.

The `syn` crate gives us lots of useful data structures, and will parse our
macro input.

The `proc-macro2` crate provides, among other things, a different
`TokenStream` type that is used by `quote` and `syn`.

Finally, `tango` is a crate for literate programming. It's what allows this
document to both be a markdown file and functioning, testable Rust code. You can
see it as Rust code here: [lib.rs](lib.rs).

A note before we get going: I find it very useful to install `cargo-expand`
and write a simple example using the proc-macro in, say, `tests/it.rs`.
Then you can run `cargo expand --test it` at any time and see what you're
producing. Also, you can just throw a `panic!` in your code as a helpful
print.

Let's get started writing `lib.rs`. We'll want a type to represent our subenum,
so let's define it.

```rust
struct Enum {
    ident: syn::Ident,
    variants: syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
}

impl Enum {
    fn new(ident: syn::Ident) -> Self {
        Self {
            ident,
            variants: syn::punctuated::Punctuated::new(),
        }
    }
}
```
An [`Ident`](https://docs.rs/syn/latest/syn/struct.Ident.html) is simply an
identifier, and we can create these from strings. It will be the name of our enum.
[`Punctuated`](https://docs.rs/syn/latest/syn/punctuated/struct.Punctuated.html)
is like a `Vec`, but will be rendered with the given token interspersed between
items, and [`Variant`](https://docs.rs/syn/latest/syn/struct.Variant.html)
represents an enum variant.

It seems like what we would want is a custom derive, but there's a catch. The
input to a derive macro does not include the other derive calls! So if we want
to, for example, inherit a `#[derive(Clone)]` on our subenum, we can't do that
using a custom derive. An attribute macro it is then!

A proc-macro crate must produce a public function from `lib.rs`, annotated with
the kind of proc-macro it is, and nothing else.

```rust
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn subenum(args: TokenStream, tokens: TokenStream) -> TokenStream {
```
The `args` are the arguments to our macro (the `Dog` in `#[subenum(Dog)]`), and
the `tokens` are everything else.

Fortunately, `syn` gives us some nice types to parse these as;
[`AttributeArgs`](https://docs.rs/syn/latest/syn/type.AttributeArgs.html) and
[`DeriveInput`](https://docs.rs/syn/latest/syn/struct.DeriveInput.html),
respectively.

```rust
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let mut input = syn::parse_macro_input!(tokens as syn::DeriveInput);
```

From the args, we can get a map of our subenums to create:

```rust
    let mut enum_map: std::collections::HashMap<syn::Ident, Enum> = args
        .into_iter()
        .map(|nested| match nested {
            syn::NestedMeta::Meta(meta) => meta,
            syn::NestedMeta::Lit(_) => panic!(),
        })
        .map(|meta| match meta {
            syn::Meta::Path(path) => path,
            _ => panic!(),
        })
        .map(|path| path.get_ident().unwrap().to_owned())
        .map(|ident| (ident.clone(), Enum::new(ident)))
        .collect();
```

Now, for each variant in the input enum, we can look up the `#[subenum()]`
attribute. If it's there, we need to look at its attributes (say, `Dog`),
and, for each one, add that variant to the entry in the map.

```rust
    let data = match input.data {
        syn::Data::Enum(ref mut data) => data,
        _ => panic!("Not an enum!"),
    };
```

One thing I love about writing proc-macros is that you can just panic
as error-handling. Any panics end up as compiler errors.

Now, we want to be able to go from a `Variant` to a list of subenum
idents.
```rust
    const SUBENUM: &str = "subenum";
    fn subenum_idents(variant: &syn::Variant) -> impl Iterator<Item = syn::Ident> + '_ {
        variant
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident(SUBENUM))
            .filter_map(|subenum_attr| subenum_attr.parse_meta().ok())
```
`Attribute::parse_meta` gives us a [`Meta`](https://docs.rs/syn/latest/syn/enum.Meta.html),
which is much nicer to work with than the `TokenStream` we would otherwise have.
```rust
            .flat_map(|meta| match meta {
                syn::Meta::List(list) => list.nested.into_iter(),
                _ => panic!("#[subenum] attributes must be supplied a list"),
            })
            .map(|nested| match nested {
                syn::NestedMeta::Meta(meta) => meta,
                syn::NestedMeta::Lit(_) => panic!("#[subenum] does not accept literals"),
            })
```
Okay, we now have an iterator over the interior `Meta` objects;
given `#[subenum(Foo, Bar)]`, this would be an iterator over
`Foo` and `Bar`. These are
[`Path`](https://docs.rs/syn/latest/syn/struct.Path.html)s,
though for use they should really be `Ident`s.
```rust
            .map(|meta| match meta {
                syn::Meta::Path(path) => path,
                _ => panic!("#[subenum] attributes take a list of identifiers"),
            })
            .flat_map(|path| path.get_ident().map(ToOwned::to_owned))
    }
```

Phew, okay that's written. We can now build-up our enums.

```rust
    for variant in &data.variants {
        for ident in subenum_idents(variant) {
            let mut var = variant.clone();
            let e = enum_map
                .get_mut(&ident)
                .expect("All subenums must be pre-declared at the top-evel attribute");
```
Now, before we shove this variant into our map, let's think a moment. If we put it
in as is, it will still have the `#[subenum]` attribute on it, and the compiler will
barf at us. So, we want to remove that. What about other attributes? We should
probably leave them in place, as a user would likely want any attributes on the
initial enum to be shared by the subenum.
```rust
            var.attrs.retain(|attr| !attr.path.is_ident(SUBENUM));
            e.variants.push(var);
        }
    }
```

Okay! We have our enum map built up, it's time to start producing output.
Let's think for a moment about what-all we need to produce.
1. The input enum: Recall, this is not a derive macro, so _none_ of the
   input will end up in code. Only our output will. Furthermore, we
   can't reproduce it directly, as it will include all of our `#[subenum]`
   attributes, so we'll have to clean it up.
2. The output enum(s): We want to produce not just want we built up in
   our enum map, but also any derives on the original, as well as its
   visibility.
3. Some `impl` blocks: We want `PartialEq` to go between our subenum and
   the original, as well as `From` to convert from the subenum to the
   original and `TryFrom` to convert from the original to the subenum,
   as conversion this way can fail.
That's a bit of a list, so let's get started! Might as well go in the
order of our list.
Step 1: Sanitize the input.
```rust
    fn sanitize_input_data(data: &mut syn::DataEnum) {
        for variant in data.variants.iter_mut() {
            // Note: This is just `Vec::drain_filter`. Let's use that once
            // stabilized.
            let mut i = 0;
            while i < variant.attrs.len() {
                if variant.attrs[i].path.is_ident(SUBENUM) {
                    variant.attrs.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }
    sanitize_input_data(data);
```

That wasn't too bad, onto step two; let's render our enums. We can do
this in two ways. Either we can use the
[`quote!`](https://docs.rs/quote/1.0.23/quote/macro.quote.html) macro
and write out the enum, iterating over the variants, or we can construct
a `DeriveInput` which quote already knows how to render. I like using
data structures when they're available, so let's go with the latter.
```rust
    impl Enum {
        fn render(&self, input: &syn::DeriveInput) -> syn::DeriveInput {
```
Let's clone the original and change the fields we're interested
in, leaving ones like attributes to be inherited.
```rust
            let mut output = input.clone();
            output.ident = self.ident.clone();
            let output_data = match &mut output.data {
                syn::Data::Enum(enum_data) => enum_data,
                _ => unreachable!(),
            };
            output_data.variants = self.variants.clone();

            output
        }
    }
    let output_enums = enum_map.values().map(|e| e.render(&input));
```

Okay, onto some `impl` blocks and we're done. We'll finally start using
the `quote` macro. We want to be able to convert from the subenum to the
original:
```rust
    let child_to_parent = enum_map.values().map(|e| {
        let sub_ident = &e.ident;
        let variant = e.variants.iter();
        let orig_ident = &input.ident;
        quote::quote! {
            impl From<#sub_ident> for #orig_ident {
                fn from(value: #sub_ident) -> Self {
                    match value {
                        #(
                            #sub_ident::#variant => #orig_ident::#variant,
                        )*
                    }
                }
            }
        }
    });
```
And from the original to the subenum. This one can fail, so we'll need
an error type. Since we're lazy, we'll just use `()` for now.
```rust
    let parent_to_child = enum_map.values().map(|e| {
        let sub_ident = &e.ident;
        let variant = e.variants.iter();
        let orig_ident = &input.ident;
        quote::quote! {
            impl TryFrom<#orig_ident> for #sub_ident {
                type Error = ();
                fn try_from(value: #orig_ident) -> Result<Self, Self::Error> {
                    match value {
                        #(
                            #orig_ident::#variant => Ok(#sub_ident::#variant),
                        )*
                        _ => Err(()),
                    }
                }
            }
        }
    });
```
Almost there! We still need to compare (our enums to eachother):
```rust
    let partial_eq = enum_map.values().map(|e| {
        let sub_ident = &e.ident;
        let variant: Vec<_> = e.variants.iter().collect();
        let orig_ident = &input.ident;
        quote::quote! {
            impl PartialEq<#orig_ident> for #sub_ident {
                fn eq(&self, rhs: &#orig_ident) -> bool {
                    match (self, rhs) {
                        #(
                            (#sub_ident::#variant, #orig_ident::#variant) => true,
                        )*
                        _ => false,
                    }
                }
            }
            impl PartialEq<#sub_ident> for #orig_ident {
                fn eq(&self, rhs: &#sub_ident) -> bool {
                    match (rhs, self) {
                        #(
                            (#sub_ident::#variant, #orig_ident::#variant) => true,
                        )*
                        _ => false,
                    }
                }
            }
        }
    });
```

Finally, we can put it all together as our macro output:
```rust
    quote::quote! {
        #input

        #(#output_enums)*

        #(#child_to_parent)*
        #(#parent_to_child)*
        #(#partial_eq)*
    }
    .into()
}
```

## Demo time

Now, we can write some tests to ensure our macro is working properly, and, more
fun, we can use `cargo expand` to see the produced code.

If we place the following code in `tests/example.rb`:

```ignore,rust
use subenum::subenum;

#[subenum(Dog, Small)]
enum Canis {
    Wolf,
    #[subenum(Dog)]
    Boxer,
    #[subenum(Dog)]
    GolderRetriever,
    Coyote,
    #[subenum(Dog, Small)]
    Westie,
}
```

and run `cargo expand --tests example`, we will see the fruits of our labor:

```ignore,rust
#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use subenum::subenum;
enum Canis {
    Wolf,
    Boxer,
    GolderRetriever,
    Coyote,
    Westie,
}
enum Small {
    Westie,
}
enum Dog {
    Boxer,
    GolderRetriever,
    Westie,
}
impl From<Small> for Canis {
    fn from(value: Small) -> Self {
        match value {
            Small::Westie => Canis::Westie,
        }
    }
}
impl From<Dog> for Canis {
    fn from(value: Dog) -> Self {
        match value {
            Dog::Boxer => Canis::Boxer,
            Dog::GolderRetriever => Canis::GolderRetriever,
            Dog::Westie => Canis::Westie,
        }
    }
}
impl TryFrom<Canis> for Small {
    type Error = ();
    fn try_from(value: Canis) -> Result<Self, Self::Error> {
        match value {
            Canis::Westie => Ok(Small::Westie),
            _ => Err(()),
        }
    }
}
impl TryFrom<Canis> for Dog {
    type Error = ();
    fn try_from(value: Canis) -> Result<Self, Self::Error> {
        match value {
            Canis::Boxer => Ok(Dog::Boxer),
            Canis::GolderRetriever => Ok(Dog::GolderRetriever),
            Canis::Westie => Ok(Dog::Westie),
            _ => Err(()),
        }
    }
}
impl PartialEq<Canis> for Small {
    fn eq(&self, rhs: &Canis) -> bool {
        match (self, rhs) {
            (Small::Westie, Canis::Westie) => true,
            _ => false,
        }
    }
}
impl PartialEq<Small> for Canis {
    fn eq(&self, rhs: &Small) -> bool {
        match (rhs, self) {
            (Small::Westie, Canis::Westie) => true,
            _ => false,
        }
    }
}
impl PartialEq<Canis> for Dog {
    fn eq(&self, rhs: &Canis) -> bool {
        match (self, rhs) {
            (Dog::Boxer, Canis::Boxer) => true,
            (Dog::GolderRetriever, Canis::GolderRetriever) => true,
            (Dog::Westie, Canis::Westie) => true,
            _ => false,
        }
    }
}
impl PartialEq<Dog> for Canis {
    fn eq(&self, rhs: &Dog) -> bool {
        match (rhs, self) {
            (Dog::Boxer, Canis::Boxer) => true,
            (Dog::GolderRetriever, Canis::GolderRetriever) => true,
            (Dog::Westie, Canis::Westie) => true,
            _ => false,
        }
    }
}
#[rustc_main]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[])
}
```
