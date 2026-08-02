use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprLit, Fields, FnArg, GenericArgument, Ident, ItemFn,
    Lit, LitStr, Path, PathArguments, Token, Type, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
};

const RPC_HANDLER_ARGUMENT_COUNT: usize = 2;
const DATECODE_DIGIT_COUNT: usize = 8;
const MAX_WIRE_NAME_BYTES: usize = 4096;

#[proc_macro_derive(Kbin, attributes(kbin))]
pub fn derive_kbin(input: TokenStream) -> TokenStream {
    match expand_kbin(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_kbin(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let visibility = input.vis.clone();
    let name = input.ident;
    let node = find_string(&input.attrs, "node")?
        .ok_or_else(|| syn::Error::new_spanned(&name, "missing #[kbin(node = \"...\")]"))?;
    validate_name(&node, &name)?;
    let Data::Struct(data) = input.data else {
        return Err(syn::Error::new_spanned(name, "Kbin supports structs only"));
    };
    let Fields::Named(fields) = data.fields else {
        return Err(syn::Error::new_spanned(name, "Kbin requires named fields"));
    };

    let builder = format_ident!("__{}KbinBuilder", name);
    let mut builder_fields = Vec::new();
    let mut descriptors = Vec::new();
    let mut ops = Vec::new();
    let mut attribute_encoders = Vec::new();
    let mut node_encoders = Vec::new();
    let mut decoders = Vec::new();
    let mut finishers = Vec::new();
    let mut saw_node = false;

    for (id, field) in fields.named.iter().enumerate() {
        let ident = field.ident.as_ref().unwrap();
        if has_flag(&field.attrs, "skip")? {
            finishers.push(quote!(#ident: ::core::default::Default::default()));
            continue;
        }
        let wire_name = find_string(&field.attrs, "rename")?.unwrap_or_else(|| ident.to_string());
        let attr = has_flag(&field.attrs, "attr")?;
        if attr && saw_node {
            return Err(syn::Error::new_spanned(
                ident,
                "attribute fields must be declared before child-node fields",
            ));
        }
        if !attr {
            saw_node = true;
        }
        validate_name(&wire_name, ident)?;
        if attr && wire_name.starts_with("__") {
            return Err(syn::Error::new_spanned(
                ident,
                "attribute names beginning with __ are reserved",
            ));
        }
        let default = has_flag(&field.attrs, "default")?;
        let (optional, inner_ty) = option_inner(&field.ty);
        let storage_ty = inner_ty.clone();
        let sequence = vec_inner(&inner_ty);
        let fixed = fixed_info(&inner_ty);
        let array = has_flag(&field.attrs, "array")?;
        let repeated = has_flag(&field.attrs, "repeated")?;
        if array && repeated {
            return Err(syn::Error::new_spanned(
                ident,
                "Vec fields cannot be both #[kbin(array)] and #[kbin(repeated)]",
            ));
        }
        if sequence.is_some() && !array && !repeated {
            return Err(syn::Error::new_spanned(
                ident,
                "Vec fields require either #[kbin(array)] or #[kbin(repeated)]",
            ));
        }
        if sequence.is_none() && (array || repeated) {
            return Err(syn::Error::new_spanned(
                ident,
                "#[kbin(array)] and #[kbin(repeated)] require a Vec field",
            ));
        }
        if attr && sequence.is_some() {
            return Err(syn::Error::new_spanned(
                ident,
                "Vec fields cannot be encoded as attributes",
            ));
        }
        let sequence_optional = sequence.is_some();
        let id = id as u16;
        let kind = if attr {
            quote!(::vibea3::schema::FieldKind::Attribute)
        } else {
            quote!(::vibea3::schema::FieldKind::Node)
        };
        let mut field_encoders = Vec::new();

        builder_fields.push(quote!(#ident: ::core::option::Option<#storage_ty>));
        descriptors.push(quote!(::vibea3::schema::Field {
            id: #id,
            name: #wire_name,
            kind: #kind,
            optional: #optional || #default || #sequence_optional,
        }));
        ops.push(quote!(::vibea3::schema::Op::Field(#id)));
        if let Some((element, len, type_id)) = fixed.as_ref() {
            if optional {
                field_encoders.push(quote! {
                    if let ::core::option::Option::Some(values) = &self.#ident {
                        encoder.fixed::<#element, #len>(#wire_name, #kind, #type_id, values)?;
                    }
                });
            } else {
                field_encoders.push(quote!(encoder.fixed::<#element, #len>(#wire_name, #kind, #type_id, &self.#ident)?;));
            }
            decoders.push(quote! {
                #id => {
                    if builder.#ident.is_some() { return ::core::result::Result::Err(::vibea3::Error::Duplicate(#wire_name)); }
                    builder.#ident = ::core::option::Option::Some(decoder.fixed::<#element, #len>(#type_id)?);
                }
            });
        } else if let Some(element) = sequence.as_ref() {
            let emit = if array {
                quote!(encoder.array(#wire_name, #kind, values)?;)
            } else {
                quote!(for value in values { encoder.field(#wire_name, #kind, value)?; })
            };
            if optional {
                field_encoders.push(quote! {
                    if let ::core::option::Option::Some(values) = &self.#ident { #emit }
                });
            } else {
                field_encoders.push(quote! { let values = &self.#ident; #emit });
            }
            if array {
                decoders.push(quote! {
                    #id => {
                        if builder.#ident.is_some() { return ::core::result::Result::Err(::vibea3::Error::Duplicate(#wire_name)); }
                        builder.#ident = ::core::option::Option::Some(decoder.array::<#element>()?);
                    }
                });
            } else {
                decoders.push(quote! {
                    #id => builder.#ident.get_or_insert_with(::std::vec::Vec::new).push(decoder.value::<#element>()?)
                });
            }
        } else if optional {
            field_encoders.push(quote! {
                if let ::core::option::Option::Some(value) = &self.#ident {
                    encoder.field(#wire_name, #kind, value)?;
                }
            });
        } else {
            field_encoders.push(quote!(encoder.field(#wire_name, #kind, &self.#ident)?;));
        }
        let field_encoder = quote!(#(#field_encoders)*);
        if attr {
            attribute_encoders.push((wire_name.clone(), field_encoder));
        } else {
            node_encoders.push(field_encoder);
        }
        if sequence.is_none() && fixed.is_none() {
            decoders.push(quote! {
                #id => {
                    if builder.#ident.is_some() {
                        return ::core::result::Result::Err(::vibea3::Error::Duplicate(#wire_name));
                    }
                    builder.#ident = ::core::option::Option::Some(decoder.value::<#storage_ty>()?);
                }
            });
        }
        if optional {
            finishers.push(quote!(#ident: builder.#ident));
        } else if sequence.is_some() || default {
            finishers.push(quote!(#ident: builder.#ident.unwrap_or_default()));
        } else {
            finishers
                .push(quote!(#ident: builder.#ident.ok_or(::vibea3::Error::Missing(#wire_name))?));
        }
    }

    attribute_encoders.sort_by(|left, right| left.0.cmp(&right.0));
    let encoders = attribute_encoders
        .into_iter()
        .map(|(_, encoder)| encoder)
        .chain(node_encoders);

    let decode_match = if decoders.is_empty() {
        quote! {
            let _ = (builder, decoder);
            return ::core::result::Result::Err(::vibea3::Error::Schema(format!("invalid field id {id}")));
        }
    } else {
        quote! {
            match id {
                #(#decoders,)*
                _ => return ::core::result::Result::Err(::vibea3::Error::Schema("invalid generated field id".into())),
            }
            ::core::result::Result::Ok(())
        }
    };

    Ok(quote! {
        #[doc(hidden)]
        #[derive(::core::default::Default)]
        #visibility struct #builder { #(#builder_fields,)* }

        impl ::vibea3::schema::Kbin for #name {
            type Builder = #builder;

            const SCHEMA: &'static ::vibea3::schema::Schema = &::vibea3::schema::Schema {
                node: #node,
                fields: &[#(#descriptors,)*],
                ops: &[
                    ::vibea3::schema::Op::Enter(#node),
                    #(#ops,)*
                    ::vibea3::schema::Op::Exit,
                ],
            };

            fn encode(&self, encoder: &mut ::vibea3::codec::Encoder<'_>) -> ::vibea3::Result<()> {
                #(#encoders)*
                ::core::result::Result::Ok(())
            }

            fn decode_field(
                builder: &mut Self::Builder,
                id: u16,
                decoder: &mut ::vibea3::codec::Decoder<'_, '_>,
            ) -> ::vibea3::Result<()> {
                #decode_match
            }

            fn finish(builder: Self::Builder) -> ::vibea3::Result<Self> {
                ::core::result::Result::Ok(Self { #(#finishers,)* })
            }
        }
    })
}

#[proc_macro_attribute]
pub fn rpc(args: TokenStream, input: TokenStream) -> TokenStream {
    let route = parse_macro_input!(args as LitStr);
    let function = parse_macro_input!(input as ItemFn);
    if function.sig.asyncness.is_none() {
        return syn::Error::new_spanned(function.sig.fn_token, "RPC handlers must be async")
            .to_compile_error()
            .into();
    }
    if function.sig.inputs.len() != RPC_HANDLER_ARGUMENT_COUNT {
        return syn::Error::new_spanned(
            &function.sig.inputs,
            "RPC handlers take RpcContext and request",
        )
        .to_compile_error()
        .into();
    }
    let value = route.value();
    let valid_route = value.split_once('.').is_some_and(|(class, method)| {
        !class.is_empty()
            && !method.is_empty()
            && !method.contains('.')
            && class.is_ascii()
            && method.is_ascii()
    });
    if !valid_route {
        return syn::Error::new_spanned(route, "route must be one ASCII class.method pair")
            .to_compile_error()
            .into();
    }
    let ident = &function.sig.ident;
    let meta = format_ident!("__vibea3_rpc_{}", ident);
    let state = match function.sig.inputs.first() {
        Some(FnArg::Typed(arg)) => rpc_context_inner(&arg.ty),
        _ => None,
    };
    let Some(state) = state else {
        return syn::Error::new_spanned(
            &function.sig.inputs,
            "first argument must be RpcContext<State>",
        )
        .to_compile_error()
        .into();
    };
    let request = match function.sig.inputs.iter().nth(1) {
        Some(FnArg::Typed(arg)) => (*arg.ty).clone(),
        _ => {
            return syn::Error::new_spanned(
                &function.sig.inputs,
                "second argument must be the request type",
            )
            .to_compile_error()
            .into();
        }
    };
    let state = qualify_from_child_module(state);
    let request = qualify_from_child_module(request);
    quote! {
        #function
        #[doc(hidden)]
        mod #meta {
            pub const ROUTE: &str = #route;
            pub type State = #state;

            fn dispatch(
                state: ::vibea3::registry::ErasedState,
                call: ::vibea3::registry::CallContext,
                incoming: ::vibea3::rpc::Incoming,
            ) -> ::vibea3::registry::HandlerFuture {
                ::vibea3::registry::erased_invoke::<State, #request, _, _, _>(
                    state,
                    call,
                    incoming,
                    super::#ident,
                )
            }

            #[::vibea3::__private::distributed_slice(::vibea3::registry::HANDLERS)]
            #[linkme(crate = ::vibea3::__private::linkme)]
            static DESCRIPTOR: ::vibea3::registry::HandlerDescriptor =
                ::vibea3::registry::HandlerDescriptor {
                    route: ROUTE,
                    package: env!("CARGO_PKG_NAME"),
                    module_path: module_path!(),
                    state_type: ::vibea3::registry::state_type::<State>,
                    dispatch,
                };
        }
    }
    .into()
}

struct ExportArgs {
    module: LitStr,
    version: LitStr,
    model: Option<LitStr>,
    datecode_min: Option<LitStr>,
    datecode_max: Option<LitStr>,
    services: Vec<LitStr>,
    host: Type,
    host_fingerprint: Expr,
    state: Type,
    init: Path,
    shutdown: Option<Path>,
}

struct ServiceList(Vec<LitStr>);

impl Parse for ServiceList {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        Ok(Self(
            Punctuated::<LitStr, Comma>::parse_terminated(&content)?
                .into_iter()
                .collect(),
        ))
    }
}

impl Parse for ExportArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut module = None;
        let mut version = None;
        let mut model = None;
        let mut datecode_min = None;
        let mut datecode_max = None;
        let mut services = None;
        let mut host = None;
        let mut host_fingerprint = None;
        let mut state = None;
        let mut init = None;
        let mut shutdown = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "module" => set_once(&mut module, input.parse()?, &key)?,
                "version" => set_once(&mut version, input.parse()?, &key)?,
                "model" => set_once(&mut model, input.parse()?, &key)?,
                "datecode_min" => set_once(&mut datecode_min, input.parse()?, &key)?,
                "datecode_max" => set_once(&mut datecode_max, input.parse()?, &key)?,
                "services" => {
                    let value: ServiceList = input.parse()?;
                    set_once(&mut services, value.0, &key)?;
                }
                "host" => set_once(&mut host, input.parse()?, &key)?,
                "host_fingerprint" => set_once(&mut host_fingerprint, input.parse()?, &key)?,
                "state" => set_once(&mut state, input.parse()?, &key)?,
                "init" => set_once(&mut init, input.parse()?, &key)?,
                "shutdown" => set_once(&mut shutdown, input.parse()?, &key)?,
                _ => return Err(syn::Error::new_spanned(key, "unknown module export option")),
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(Self {
            module: required(module, "module")?,
            version: required(version, "version")?,
            model,
            datecode_min,
            datecode_max,
            services: services.unwrap_or_default(),
            host: required(host, "host")?,
            host_fingerprint: required(host_fingerprint, "host_fingerprint")?,
            state: required(state, "state")?,
            init: required(init, "init")?,
            shutdown,
        })
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, key: &Ident) -> syn::Result<()> {
    if slot.replace(value).is_some() {
        return Err(syn::Error::new_spanned(
            key,
            "duplicate module export option",
        ));
    }
    Ok(())
}

fn required<T>(value: Option<T>, name: &str) -> syn::Result<T> {
    value.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("missing `{name}: ...` in export_xrpc_module!"),
        )
    })
}

#[proc_macro]
pub fn export_xrpc_module(input: TokenStream) -> TokenStream {
    let ExportArgs {
        module,
        version,
        model,
        datecode_min,
        datecode_max,
        services,
        host,
        host_fingerprint,
        state,
        init,
        shutdown,
    } = parse_macro_input!(input as ExportArgs);
    if let Some(model) = &model
        && (model.value().is_empty()
            || !model.value().is_ascii()
            || !model
                .value()
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    {
        return syn::Error::new_spanned(
            model,
            "model must be a non-empty ASCII product identifier",
        )
        .to_compile_error()
        .into();
    }
    for datecode in [&datecode_min, &datecode_max].into_iter().flatten() {
        if datecode.value().len() != DATECODE_DIGIT_COUNT
            || !datecode.value().bytes().all(|byte| byte.is_ascii_digit())
        {
            return syn::Error::new_spanned(datecode, "datecode must contain exactly 8 digits")
                .to_compile_error()
                .into();
        }
    }
    if let (Some(min), Some(max)) = (&datecode_min, &datecode_max)
        && min.value() > max.value()
    {
        return syn::Error::new_spanned(max, "datecode_max must not be less than datecode_min")
            .to_compile_error()
            .into();
    }
    let mut unique_services = ::std::collections::BTreeSet::new();
    for service in &services {
        let value = service.value();
        if value.is_empty()
            || !value.is_ascii()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return syn::Error::new_spanned(
                service,
                "service names must be non-empty ASCII identifiers",
            )
            .to_compile_error()
            .into();
        }
        if !unique_services.insert(value) {
            return syn::Error::new_spanned(service, "duplicate service name")
                .to_compile_error()
                .into();
        }
    }
    let model = optional_literal(model.as_ref());
    let datecode_min = optional_literal(datecode_min.as_ref());
    let datecode_max = optional_literal(datecode_max.as_ref());
    let shutdown_body = if let Some(shutdown) = shutdown {
        quote! {
            #shutdown(state).await.map_err(|error| error.to_string())
        }
    } else {
        quote! {
            let _ = state;
            ::core::result::Result::Ok(())
        }
    };
    quote! {
        #[cfg(not(panic = "unwind"))]
        compile_error!("dynamic XRPC modules require panic = \"unwind\"");

        #[doc(hidden)]
        fn __vibea3_module_host_type() -> &'static str {
            ::core::any::type_name::<#host>()
        }

        #[doc(hidden)]
        fn __vibea3_module_state_type() -> &'static str {
            ::core::any::type_name::<#state>()
        }

        #[doc(hidden)]
        fn __vibea3_module_handlers() -> ::core::result::Result<
            ::std::vec::Vec<::vibea3::registry::HandlerDescriptor>,
            ::std::string::String,
        > {
            ::vibea3::registry::registered_handlers(
                env!("CARGO_PKG_NAME"),
                __vibea3_module_state_type(),
            )
            .map(|handlers| handlers.into_iter().copied().collect())
            .map_err(|error| error.to_string())
        }

        #[doc(hidden)]
        unsafe fn __vibea3_module_init(
            host: *const (),
        ) -> ::vibea3::module::InitFuture {
            let host = unsafe { (&*(host as *const ::std::sync::Arc<#host>)).clone() };
            ::vibea3::module::guard_init(async move {
                #init(host)
                    .await
                    .map(|state| {
                        ::std::sync::Arc::new(state) as ::vibea3::registry::ErasedState
                    })
                    .map_err(|error| error.to_string())
            })
        }

        #[doc(hidden)]
        unsafe fn __vibea3_module_shutdown(
            state: ::vibea3::registry::ErasedState,
        ) -> ::vibea3::module::ShutdownFuture {
            ::vibea3::module::guard_shutdown(async move {
                let state = state
                    .as_ref()
                    .downcast_ref::<#state>()
                    .ok_or_else(|| "module state type changed during shutdown".to_owned())?
                    .clone();
                #shutdown_body
            })
        }

        #[doc(hidden)]
        static __VIBEA3_MODULE_EXPORT: ::vibea3::module::ModuleExport =
            ::vibea3::module::ModuleExport {
                magic: ::vibea3::module::MODULE_MAGIC,
                abi_version: ::vibea3::module::MODULE_ABI_VERSION,
                struct_size: ::core::mem::size_of::<::vibea3::module::ModuleExport>(),
                vibea3_version: ::vibea3::module::VIBEA3_VERSION,
                build_fingerprint: ::vibea3::module::build_fingerprint,
                module_id: #module,
                module_version: #version,
                model: #model,
                datecode_min: #datecode_min,
                datecode_max: #datecode_max,
                services: &[#(#services),*],
                package: env!("CARGO_PKG_NAME"),
                host_type: __vibea3_module_host_type,
                host_fingerprint: #host_fingerprint,
                state_type: __vibea3_module_state_type,
                handlers: __vibea3_module_handlers,
                init: __vibea3_module_init,
                shutdown: __vibea3_module_shutdown,
            };

        #[unsafe(no_mangle)]
        pub extern "C" fn vibea3_xrpc_module_v2() -> *const ::vibea3::module::ModuleExport {
            &__VIBEA3_MODULE_EXPORT
        }
    }
    .into()
}

fn optional_literal(value: Option<&LitStr>) -> proc_macro2::TokenStream {
    match value {
        Some(value) => quote!(::core::option::Option::Some(#value)),
        None => quote!(::core::option::Option::None),
    }
}

#[proc_macro]
pub fn rpc_app(input: TokenStream) -> TokenStream {
    let handlers = parse_macro_input!(input with Punctuated::<Ident, Comma>::parse_terminated);
    let Some(first) = handlers.first() else {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "rpc_app! requires at least one handler",
        )
        .to_compile_error()
        .into();
    };
    let first_meta = format_ident!("__vibea3_rpc_{}", first);
    let arms = handlers.iter().map(|handler| {
        let meta = format_ident!("__vibea3_rpc_{}", handler);
        quote!(#meta::ROUTE => ::vibea3::rpc::invoke(ctx, incoming, #handler).await,)
    });
    let routes = handlers.iter().map(|handler| {
        let meta = format_ident!("__vibea3_rpc_{}", handler);
        quote!(#meta::ROUTE)
    });
    quote! {{
        const _: () = {
            const ROUTES: &[&str] = &[#(#routes,)*];
            const fn same(left: &str, right: &str) -> bool {
                let left = left.as_bytes();
                let right = right.as_bytes();
                if left.len() != right.len() { return false; }
                let mut index = 0;
                while index < left.len() {
                    if left[index] != right[index] { return false; }
                    index += 1;
                }
                true
            }
            let mut left = 0;
            while left < ROUTES.len() {
                let mut right = left + 1;
                while right < ROUTES.len() {
                    if same(ROUTES[left], ROUTES[right]) { panic!("duplicate RPC route"); }
                    right += 1;
                }
                left += 1;
            }
        };
        struct __Vibea3StaticApp;
        impl ::vibea3::rpc::RpcDispatch<#first_meta::State> for __Vibea3StaticApp {
            async fn dispatch(
                &self,
                route: &str,
                ctx: ::vibea3::rpc::RpcContext<#first_meta::State>,
                incoming: ::vibea3::rpc::Incoming,
            ) -> ::vibea3::rpc::RpcResult<::vibea3::rpc::Outgoing> {
                match route {
                    #(#arms)*
                    _ => ::core::result::Result::Err(::vibea3::rpc::RpcError::method_not_found(route)),
                }
            }
        }
        __Vibea3StaticApp
    }}.into()
}

fn option_inner(ty: &Type) -> (bool, Type) {
    let Type::Path(path) = ty else {
        return (false, ty.clone());
    };
    let Some(segment) = path.path.segments.last() else {
        return (false, ty.clone());
    };
    if segment.ident != "Option" {
        return (false, ty.clone());
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return (false, ty.clone());
    };
    let Some(GenericArgument::Type(inner)) = args.args.first() else {
        return (false, ty.clone());
    };
    (true, inner.clone())
}

fn vec_inner(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Vec" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    }
}

fn fixed_info(ty: &Type) -> Option<(Type, usize, u8)> {
    let Type::Array(array) = ty else {
        return None;
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Int(len), ..
    }) = &array.len
    else {
        return None;
    };
    let len = len.base10_parse::<usize>().ok()?;
    let element = (*array.elem).clone();
    let Type::Path(path) = &element else {
        return None;
    };
    let name = path.path.segments.last()?.ident.to_string();
    let id = match (name.as_str(), len) {
        ("i8", 2) => 0x10,
        ("u8", 2) => 0x11,
        ("i16", 2) => 0x12,
        ("u16", 2) => 0x13,
        ("i32", 2) => 0x14,
        ("u32", 2) => 0x15,
        ("i64", 2) => 0x16,
        ("u64", 2) => 0x17,
        ("f32", 2) => 0x18,
        ("f64", 2) => 0x19,
        ("bool", 2) => 0x35,
        ("i8", 3) => 0x1a,
        ("u8", 3) => 0x1b,
        ("i16", 3) => 0x1c,
        ("u16", 3) => 0x1d,
        ("i32", 3) => 0x1e,
        ("u32", 3) => 0x1f,
        ("i64", 3) => 0x20,
        ("u64", 3) => 0x21,
        ("f32", 3) => 0x22,
        ("f64", 3) => 0x23,
        ("bool", 3) => 0x36,
        ("i8", 4) => 0x24,
        ("u8", 4) => 0x25,
        ("i16", 4) => 0x26,
        ("u16", 4) => 0x27,
        ("i32", 4) => 0x28,
        ("u32", 4) => 0x29,
        ("i64", 4) => 0x2a,
        ("u64", 4) => 0x2b,
        ("f32", 4) => 0x2c,
        ("f64", 4) => 0x2d,
        ("bool", 4) => 0x37,
        ("i8", 16) => 0x30,
        ("u8", 16) => 0x31,
        ("i16", 8) => 0x32,
        ("u16", 8) => 0x33,
        ("bool", 16) => 0x38,
        _ => return None,
    };
    Some((element, len, id))
}

fn rpc_context_inner(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "RpcContext" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(ty) => Some(ty.clone()),
        _ => None,
    }
}

fn qualify_from_child_module(ty: Type) -> proc_macro2::TokenStream {
    if let Type::Path(path) = &ty {
        let first = path.path.segments.first().map(|s| s.ident.to_string());
        if path.qself.is_none()
            && path.path.leading_colon.is_none()
            && !matches!(first.as_deref(), Some("crate" | "self" | "super"))
        {
            return quote!(super::#ty);
        }
    }
    quote!(#ty)
}

fn validate_name(name: &str, span: impl quote::ToTokens) -> syn::Result<()> {
    if name.is_empty() || name.len() > MAX_WIRE_NAME_BYTES {
        return Err(syn::Error::new_spanned(
            span,
            "wire names must contain 1..=4096 bytes",
        ));
    }
    if !name.is_ascii() {
        return Err(syn::Error::new_spanned(span, "v1 wire names must be ASCII"));
    }
    Ok(())
}

fn find_string(attrs: &[Attribute], key: &str) -> syn::Result<Option<String>> {
    let mut found = None;
    for attr in attrs.iter().filter(|a| a.path().is_ident("kbin")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                found = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.input.peek(Token![=]) {
                let _ = meta.value()?.parse::<syn::Expr>()?;
            }
            Ok(())
        })?;
    }
    Ok(found)
}

fn has_flag(attrs: &[Attribute], key: &str) -> syn::Result<bool> {
    let mut found = false;
    for attr in attrs.iter().filter(|a| a.path().is_ident("kbin")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                found = true;
            }
            if meta.input.peek(Token![=]) {
                let _ = meta.value()?.parse::<syn::Expr>()?;
            }
            Ok(())
        })?;
    }
    Ok(found)
}
