use proc_macro::Span;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse_macro_input, parse_str, spanned::Spanned, Attribute, Data, DeriveInput, Expr, Fields,
    GenericParam, Lit, LitByteStr, LitInt, Meta, NestedMeta, Path,
};

fn meta_ident_eq(path: &Path, value: &str) -> bool {
    if path.segments.len() != 1 {
        return false;
    }
    path.segments.first().unwrap().ident == value
}

fn parse_expr_in_str_literal(literal: &Lit) -> Option<TokenStream> {
    let lit = match &literal {
        Lit::Str(lit) => lit,
        _ => return None,
    };

    let expr: Expr = parse_str(&lit.value()).ok()?;
    Some(quote_spanned! {lit.span()=>
        #expr
    })
}

#[derive(Default)]
struct LoadStoreOptions {
    pre_store: Option<TokenStream>,
    post_store: Option<TokenStream>,
    post_load: Option<TokenStream>,
    only_load_in_place: bool,
}

impl LoadStoreOptions {
    fn parse(attrs: &[Attribute]) -> Result<Self, &str> {
        let mut options = LoadStoreOptions::default();

        for attr in attrs {
            let meta_list = match attr.parse_meta() {
                Ok(Meta::List(meta_list)) => meta_list,
                _ => continue,
            };

            macro_rules! parse_fns {
                (
                    $name: literal,
                    $(($pre_post: literal, $fn_ident: ident)),*
                    $(; $only_load_in_place: literal)?
                ) => {
                    for nested_meta in meta_list.nested.iter() {
                        let meta = match nested_meta {
                            NestedMeta::Meta(meta) => meta,
                            _ => return Err(concat!("invalid `", $name, "` attribute")),
                        };
                        match meta {
                            Meta::NameValue(name_value) => {
                                $(if meta_ident_eq(&name_value.path, $pre_post) {
                                    options.$fn_ident = Some(
                                        parse_expr_in_str_literal(&name_value.lit).ok_or(concat!(
                                            "invalid ",
                                            $pre_post,
                                            "-",
                                            $name,
                                            " code specification"
                                        ))?,
                                    );
                                } else)* {
                                    return Err(concat!("invalid `", $name, "` attribute"));
                                }
                            }
                            $(Meta::Path(path) => {
                                if meta_ident_eq(path, $only_load_in_place) {
                                    options.only_load_in_place = true;
                                }
                            })*
                            _ => {}
                        }
                    }
                };
            }

            if meta_ident_eq(&meta_list.path, "store") {
                parse_fns!("store", ("pre", pre_store), ("post", post_store));
            } else if meta_ident_eq(&meta_list.path, "load") {
                parse_fns!("load", ("post", post_load); "in_place_only");
            }
        }

        Ok(options)
    }
}

struct FieldsData {
    load: Option<Vec<TokenStream>>,
    load_in_place: Option<Vec<TokenStream>>,
    store: Vec<TokenStream>,
}

#[derive(Clone)]
enum LoadStoreKind {
    Value(TokenStream),
    Fn(TokenStream),
    Default,
}

impl FieldsData {
    fn parse(fields: &Fields, only_load: bool, mut only_load_in_place: bool) -> Result<Self, &str> {
        let fields_and_idents = match fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|field| {
                    let ident = field.ident.clone().unwrap();
                    (
                        Some(LitByteStr::new(
                            &ident.to_string().into_bytes(),
                            ident.span(),
                        )),
                        ident,
                        field,
                    )
                })
                .collect::<Vec<_>>(),

            Fields::Unnamed(unnamed) => (unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, field)| (None, format_ident!("f{}", i), field)))
            .collect(),

            Fields::Unit => Vec::new(),
        };

        let mut load = Vec::new();
        let mut load_in_place = Vec::new();
        let mut store = Vec::new();

        for (name, ident, field) in fields_and_idents {
            let mut load_kind = Some(LoadStoreKind::Default);
            let mut load_in_place_kind = Some(LoadStoreKind::Default);
            let mut store_kind = Some(LoadStoreKind::Default);

            for attr in &field.attrs {
                let meta_list = match attr.parse_meta() {
                    Ok(Meta::List(meta_list)) => meta_list,
                    _ => continue,
                };

                macro_rules! parse_exprs {
                    ($name: literal, $kind: ident $(, $in_place_kind: ident)?) => {
                        for nested_meta in meta_list.nested.iter() {
                            let meta = match nested_meta {
                                NestedMeta::Meta(meta) => meta,
                                _ => return Err(concat!("invalid `", $name, "` attribute")),
                            };
                            match meta {
                                Meta::Path(word) => {
                                    if meta_ident_eq(word, "skip") {
                                        $kind = None;
                                        $($in_place_kind = None;)*
                                    }
                                }

                                Meta::NameValue(name_value) => {
                                    if meta_ident_eq(&name_value.path, "value") {
                                        $kind = Some(LoadStoreKind::Value(
                                            parse_expr_in_str_literal(&name_value.lit).ok_or(
                                                concat!("invalid ", $name, " value specification"),
                                            )?,
                                        ));
                                        $($in_place_kind = $kind.clone();)*
                                    } else if meta_ident_eq(&name_value.path, "with") {
                                        let expr =
                                            parse_expr_in_str_literal(&name_value.lit).ok_or(
                                                concat!("invalid ", $name, " value specification"),
                                            )?;
                                        $kind = Some(LoadStoreKind::Fn(
                                            quote_spanned! {expr.span()=>
                                                #[allow(unused_variables)]
                                                let save = &mut *save;
                                                #expr
                                            },
                                        ));
                                    } $(
                                        else if meta_ident_eq(&name_value.path, "with_in_place") {
                                            let expr = parse_expr_in_str_literal(&name_value.lit)
                                                .ok_or(concat!(
                                                    "invalid ",
                                                    $name,
                                                    " value specification",
                                                ))?;
                                            $in_place_kind = Some(LoadStoreKind::Fn(
                                                quote_spanned! {expr.span()=>
                                                    #[allow(unused_variables)]
                                                    let save = &mut *save;
                                                    #expr
                                                },
                                            ));
                                        }
                                    )*
                                }
                                _ => {}
                            }
                        }
                    };
                }

                if meta_ident_eq(&meta_list.path, "load") {
                    parse_exprs!("load", load_kind, load_in_place_kind);
                } else if meta_ident_eq(&meta_list.path, "store") {
                    parse_exprs!("store", store_kind);
                } else if meta_ident_eq(&meta_list.path, "savestate") {
                    for nested_meta in meta_list.nested.iter() {
                        let meta = match nested_meta {
                            NestedMeta::Meta(meta) => meta,
                            _ => return Err(concat!("invalid `savestate` attribute")),
                        };
                        if let Meta::Path(word) = meta {
                            if meta_ident_eq(word, "skip") {
                                load_kind = None;
                                load_in_place_kind = None;
                                store_kind = None;
                            }
                        }
                    }
                }
            }

            if load_kind.is_none() {
                if only_load {
                    panic!("skipping field loads is disallowed in this context");
                }
                only_load_in_place = true;
                load.clear();
            }

            if matches!(&load_kind, Some(LoadStoreKind::Fn(_)))
                != matches!(&load_in_place_kind, Some(LoadStoreKind::Fn(_)))
                && !((matches!(
                    &load_kind,
                    Some(LoadStoreKind::Default | LoadStoreKind::Value(_))
                ) && only_load_in_place)
                    || (matches!(
                        &load_in_place_kind,
                        Some(LoadStoreKind::Default | LoadStoreKind::Value(_))
                    ) && only_load))
            {
                panic!(concat!(
                    "if one of #[load(with = \"...\")] or #[load(with_in_place = \"...\")] is ",
                    "used, the other must be present too",
                ));
            }

            if let Some(store_kind) = store_kind {
                let store_expr = match store_kind {
                    LoadStoreKind::Default => {
                        quote_spanned! {ident.span()=>
                            save.store(#ident)?
                        }
                    }

                    LoadStoreKind::Value(value) => {
                        quote_spanned! {ident.span()=>
                            save.store(#value)?
                        }
                    }

                    LoadStoreKind::Fn(value) => value,
                };

                let name = name.as_ref().into_iter();
                store.push(quote_spanned! {ident.span()=>
                    {
                        #(save.start_field(#name)?;)*
                        #store_expr;
                    }
                });
            }

            if let Some(load_kind) = load_kind {
                if !only_load {
                    let name = name.as_ref().into_iter();
                    load_in_place.push(match load_in_place_kind.unwrap() {
                        LoadStoreKind::Default => {
                            quote_spanned! {ident.span()=> {
                                #(save.start_field(#name)?;)*
                                save.load_into(#ident)?;
                            }}
                        }

                        LoadStoreKind::Value(value) => {
                            quote_spanned! {ident.span()=>
                                *#ident = #value;
                            }
                        }

                        LoadStoreKind::Fn(value) => {
                            quote_spanned! {ident.span()=> {
                                #(save.start_field(#name)?;)*
                                #value;
                            }}
                        }
                    });
                }

                if !only_load_in_place {
                    let name = name.as_ref().into_iter();
                    load.push(match load_kind {
                        LoadStoreKind::Default => {
                            quote_spanned! {ident.span()=> {
                                #(save.start_field(#name)?;)*
                                save.load()?
                            }}
                        }

                        LoadStoreKind::Value(value) => {
                            quote_spanned!(ident.span()=> {#value})
                        },
                        
                        LoadStoreKind::Fn(value) => 
                        quote_spanned! {ident.span()=> {
                            #(save.start_field(#name)?;)*
                            #value
                        }},
                    });
                }
            }
        }

        Ok(FieldsData {
            store,
            load: if only_load_in_place { None } else { Some(load) },
            load_in_place: if only_load { None } else { Some(load_in_place) },
        })
    }
}

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let (store_where_clause, load_in_place_where_clause, load_where_clause) =
        if input.generics.params.is_empty() {
            (quote!(), quote!(), quote!())
        } else {
            let where_clause_start = if let Some(where_clause) = where_clause {
                quote!(#where_clause, )
            } else {
                quote!(where)
            };
            let type_params_0 = input.generics.params.iter().filter_map(|p| {
                if let GenericParam::Type(p) = p {
                    Some(&p.ident)
                } else {
                    None
                }
            });
            let type_params_1 = type_params_0.clone();
            let type_params_2 = type_params_0.clone();
            (
                quote!(#where_clause_start #(#type_params_0: ::emu_utils::Storable),*),
                quote!(#where_clause_start #(#type_params_1: ::emu_utils::LoadableInPlace),*),
                quote!(#where_clause_start #(#type_params_2: ::emu_utils::Loadable),*),
            )
        };

    let LoadStoreOptions {
        pre_store,
        post_store,
        post_load,
        only_load_in_place,
    } = LoadStoreOptions::parse(&input.attrs).unwrap_or_else(|message| panic!("{}", message));

    match &input.data {
        Data::Struct(data) => {
            let FieldsData {
                store,
                load_in_place,
                load,
            } = FieldsData::parse(&data.fields, false, only_load_in_place)
                .unwrap_or_else(|message| panic!("{}", message));

            let store_fields = store.into_iter().map(proc_macro2::TokenStream::from);
            let load_fields_in_place = load_in_place
                .unwrap()
                .into_iter()
                .map(proc_macro2::TokenStream::from);
            let load_fields = load.map(|load| load.into_iter().map(proc_macro2::TokenStream::from));

            let post_load_ident = post_load
                .as_ref()
                .map(|_| format_ident!("post_load"))
                .into_iter();
            let post_load_ident_ = post_load_ident.clone();
            let load_post_load = quote! {
                #(
                    impl #impl_generics #type_name #ty_generics #load_where_clause {
                        fn #post_load_ident<S__: ::emu_utils::ReadSavestate>(
                            &mut self,
                            save: &mut S__,
                        ) -> Result<(), S__::Error> {
                            #post_load
                        }
                    }
                )*
            };

            let (store_fields, load_fields_in_place, load_fields) = match &data.fields {
                Fields::Named(fields) => {
                    let struct_fields_0 = fields
                        .named
                        .iter()
                        .map(|field| field.ident.as_ref().unwrap());
                    let struct_fields_1 = struct_fields_0.clone();
                    let struct_fields_2 = struct_fields_0.clone();
                    let struct_fields_3 = struct_fields_0.clone();
                    (
                        quote! {
                            let #type_name { #(#struct_fields_0),* } = self;
                            save.start_struct()?;
                            #pre_store;
                            let #type_name { #(#struct_fields_1),* } = self;
                            #(#store_fields;)*
                            #post_store;
                            save.end_struct()?;
                        },
                        quote! {
                            let #type_name { #(#struct_fields_2),* } = self;
                            save.start_struct()?;
                            #(#load_fields_in_place;)*
                            #post_load;
                            save.end_struct()?;
                        },
                        load_fields.map(|load_fields| {
                            quote! {
                                save.start_struct()?;
                                let mut value = #type_name {
                                    #(#struct_fields_3: #load_fields),*
                                };
                                #(value.#post_load_ident_();)*
                                save.end_struct()?;
                                Ok(value)
                            }
                        }),
                    )
                }

                Fields::Unnamed(fields) => {
                    let struct_fields_0 =
                        (0..fields.unnamed.len()).map(|i| format_ident!("f{}", i));
                    let struct_fields_1 = struct_fields_0.clone();
                    let struct_fields_2 = struct_fields_0.clone();
                    (
                        quote! {
                            let #type_name(#(#struct_fields_0),*) = self;
                            #pre_store;
                            let #type_name(#(#struct_fields_1),*) = self;
                            #(#store_fields;)*
                            #post_store;
                        },
                        quote! {
                            let #type_name(#(#struct_fields_2),*) = self;
                            #(#load_fields_in_place;)*
                            #post_load;
                        },
                        load_fields.map(|load_fields| {
                            quote! {
                                let mut value = #type_name(#(#load_fields),*);
                                #(value.#post_load_ident_();)*
                                Ok(value)
                            }
                        }),
                    )
                }

                Fields::Unit => (
                    quote! {
                        #pre_store;
                        #post_store;
                    },
                    quote! {
                        #post_load;
                    },
                    Some(quote! {
                        let mut value = #type_name;
                        #(value.#post_load_ident_();)*
                        Ok(value)
                    }),
                ),
            };

            let storable_impl = quote! {
                #[allow(unused_variables)]
                impl #impl_generics ::emu_utils::Storable for #type_name #ty_generics
                    #store_where_clause
                {
                    fn store<S__: ::emu_utils::WriteSavestate>(
                        &mut self,
                        save: &mut S__,
                    ) -> Result<(), S__::Error> {
                        #store_fields
                        Ok(())
                    }
                }
            };

            let loadable_in_place_impl = quote! {
                #[allow(unused_variables)]
                impl #impl_generics ::emu_utils::LoadableInPlace for #type_name #ty_generics
                    #load_in_place_where_clause
                {
                    fn load_in_place<S__: ::emu_utils::ReadSavestate>(
                        &mut self,
                        save: &mut S__,
                    ) -> Result<(), S__::Error> {
                        #load_fields_in_place
                        Ok(())
                    }
                }
            };

            let loadable_impl = load_fields
                .map(|load_fields| {
                    quote! {
                        #load_post_load

                        impl #impl_generics ::emu_utils::Loadable for #type_name #ty_generics
                            #load_where_clause
                        {
                            fn load<S__: ::emu_utils::ReadSavestate>(
                                save: &mut S__,
                            ) -> Result<Self, S__::Error> {
                                #load_fields
                            }
                        }
                    }
                })
                .unwrap_or_else(|| quote!());

            quote! {
                #storable_impl
                #loadable_in_place_impl
                #loadable_impl
            }
            .into()
        }

        Data::Enum(data) => {
            let discr_bits = (32
                - (u32::try_from(data.variants.len()).expect("too many variants")).leading_zeros())
            .next_power_of_two()
            .max(8);
            let discr_ty = format_ident!("u{}", discr_bits);

            let variants_data = data
                .variants
                .iter()
                .enumerate()
                .map(|(discr, variant)| {
                    let discr_literal = Lit::Int(LitInt::new(
                        &format!("{}_u{}", discr, discr_bits),
                        Span::call_site().into(),
                    ));

                    let FieldsData {
                        store,
                        load_in_place,
                        load,
                    } = FieldsData::parse(&variant.fields, !only_load_in_place, only_load_in_place)
                        .unwrap_or_else(|message| panic!("{}", message));

                    let store_fields = store.into_iter().map(proc_macro2::TokenStream::from);
                    let load_fields_in_place = load_in_place.map(|load_in_place| {
                        load_in_place
                            .into_iter()
                            .map(proc_macro2::TokenStream::from)
                    });
                    let load_fields =
                        load.map(|load| load.into_iter().map(proc_macro2::TokenStream::from));

                    let variant_name = &variant.ident;

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let variant_fields_0 = fields
                                .named
                                .iter()
                                .map(|field| field.ident.as_ref().unwrap());
                            let variant_fields_1 = variant_fields_0.clone();
                            (
                                quote! {
                                    #type_name::#variant_name {
                                        #(#variant_fields_0),*
                                    } => {
                                        save.store_raw(#discr_literal);
                                        save.start_struct()?;
                                        #(#store_fields;)*
                                        save.end_struct()?;
                                    }
                                },
                                if only_load_in_place {
                                    let load_fields_in_place = load_fields_in_place.unwrap();
                                    quote! {
                                        #discr_literal => {
                                            if let #type_name::#variant_name {
                                                #(#variant_fields_1),*
                                            } = self {
                                                save.start_struct()?;
                                                #(#load_fields_in_place;)*
                                                save.end_struct()?;
                                            } else {
                                                return Err(S__::invalid_enum());
                                            }
                                        }
                                    }
                                } else {
                                    let load_fields = load_fields.unwrap();
                                    quote! {
                                        #discr_literal => {
                                            save.start_struct()?;
                                            let value = #type_name::#variant_name {
                                                #(#variant_fields_1: #load_fields),*
                                            };
                                            save.end_struct()?;
                                            value
                                        }
                                    }
                                },
                            )
                        }

                        Fields::Unnamed(fields) => {
                            let variant_fields_0 =
                                (0..fields.unnamed.len()).map(|i| format_ident!("f{}", i));
                            let variant_fields_1 = variant_fields_0.clone();
                            (
                                quote! {
                                    #type_name::#variant_name(#(#variant_fields_0),*) => {
                                        save.store_raw(#discr_literal);
                                        #(#store_fields;)*
                                    }
                                },
                                if only_load_in_place {
                                    let load_fields_in_place = load_fields_in_place.unwrap();
                                    quote! {
                                        #discr_literal => {
                                            if let #type_name::#variant_name(
                                                #(#variant_fields_1),*
                                            ) = self {
                                                #(#load_fields_in_place;)*
                                            } else {
                                                return Err(S__::invalid_enum());
                                            }
                                        }
                                    }
                                } else {
                                    let load_fields = load_fields.unwrap();
                                    quote! {
                                        #discr_literal => {
                                            #type_name::#variant_name(#(#load_fields),*)
                                        }
                                    }
                                },
                            )
                        }

                        Fields::Unit => (
                            quote! {
                                #type_name::#variant_name => {
                                    save.store_raw(#discr_literal);
                                }
                            },
                            if only_load_in_place {
                                quote! {
                                    #discr_literal => {
                                        if !matches!(self, #type_name::#variant_name) {
                                            return Err(S__::invalid_enum());
                                        }
                                    }
                                }
                            } else {
                                quote! {
                                    #discr_literal => {
                                        #type_name::#variant_name
                                    }
                                }
                            },
                        ),
                    }
                })
                .collect::<Vec<_>>();

            let store_variants = variants_data
                .iter()
                .map(|(store_variants, _)| store_variants);
            let storable_impl = quote! {
                #[allow(unused_variables)]
                impl #impl_generics ::emu_utils::Storable for #type_name #ty_generics
                    #store_where_clause
                {
                    fn store<S__: ::emu_utils::WriteSavestate>(
                        &mut self,
                        save: &mut S__,
                    ) -> Result<(), S__::Error> {
                        #pre_store;
                        match self {
                            #(#store_variants)*
                        }
                        #post_store;
                        Ok(())
                    }
                }
            };

            let load_variants = variants_data.iter().map(|(_, load_variants)| load_variants);
            let loadable_impl = if only_load_in_place {
                quote! {
                    #[allow(unused_variables)]
                    impl #impl_generics ::emu_utils::LoadableInPlace for #type_name #ty_generics
                        #load_in_place_where_clause
                    {
                        fn load_in_place<S__: ::emu_utils::ReadSavestate>(
                            &mut self,
                            save: &mut S__,
                        ) -> Result<(), S__::Error> {
                            let discriminant = save.load_raw::<#discr_ty>()?;
                            match discriminant {
                                #(#load_variants)*
                                _ => return Err(S__::invalid_enum()),
                            };
                            #post_load;
                            Ok(())
                        }
                    }
                }
            } else {
                let post_load_ident = post_load
                    .as_ref()
                    .map(|_| format_ident!("post_load"))
                    .into_iter();
                let post_load_ident_ = post_load_ident.clone();
                quote! {
                    #(
                        impl #impl_generics #type_name #ty_generics #load_where_clause {
                            fn #post_load_ident<S__: ::emu_utils::ReadSavestate>(
                                &mut self,
                                save: &mut S__,
                            ) -> Result<(), S__::Error> {
                                #post_load
                                Ok(())
                            }
                        }
                    )*

                    #[allow(unused_variables)]
                    impl #impl_generics ::emu_utils::Loadable for #type_name #ty_generics
                        #load_where_clause
                    {
                        fn load<S__: ::emu_utils::ReadSavestate>(
                            save: &mut S__,
                        ) -> Result<Self, S__::Error> {
                            let discriminant = save.load_raw::<#discr_ty>()?;
                            let mut value = match discriminant {
                                #(#load_variants)*
                                _ => return Err(S__::invalid_enum()),
                            };
                            #(value.#post_load_ident_();)*
                            Ok(value)
                        }
                    }

                    impl #impl_generics ::emu_utils::LoadableInPlace for #type_name #ty_generics
                        #load_where_clause
                    {
                        fn load_in_place<S__: ::emu_utils::ReadSavestate>(
                            &mut self,
                            save: &mut S__,
                        ) -> Result<(), S__::Error> {
                            *self = save.load()?;
                            Ok(())
                        }
                    }
                }
            };

            quote! {
                #storable_impl
                #loadable_impl
            }
            .into()
        }
        Data::Union(_) => unimplemented!("can't derive SavestateCapable on unions"),
    }
}
