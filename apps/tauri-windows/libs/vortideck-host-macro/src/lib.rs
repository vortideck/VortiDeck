/**
 * VortiDeck Host Function Proc-Macro
 *
 * Automatically generates:
 * - extism::Function::new() wrappers with JSON handling
 * - Schema metadata for SDK generation
 * - Type definitions
 *
 * Usage:
 * ```rust
 * /// Log a message from the plugin
 * #[vortideck_host]
 * fn vortideck_log(message: String) -> Result<bool, String> {
 *     println!("🔌 {}", message);
 *     Ok(true)
 * }
 * ```
 *
 * Generates:
 * - `vortideck_log_function()` that returns `extism::Function`
 * - JSON serialization/deserialization
 * - Metadata for build.rs extraction
 */
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Pat, ReturnType};

/// Main macro - annotates host functions and generates Function::new() boilerplate
#[proc_macro_attribute]
pub fn vortideck_host(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Extract function metadata
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let wrapper_name = syn::Ident::new(&format!("{}_function", fn_name_str), fn_name.span());
    let impl_name = syn::Ident::new(&format!("{}_impl", fn_name_str), fn_name.span());

    // Extract doc comments for description
    let description = extract_doc_comment(&input_fn.attrs);

    // Extract parameters (skip self/receiver parameters)
    let params_info = extract_parameters(&input_fn.sig.inputs);

    // Extract return type
    let return_type_str = extract_return_type(&input_fn.sig.output);

    // Generate parameter struct for JSON deserialization
    let param_struct_name = syn::Ident::new(
        &format!("{}Params", capitalize(&fn_name_str)),
        fn_name.span(),
    );

    // Generate struct fields
    let param_fields: Vec<_> = params_info
        .iter()
        .map(|p| {
            let name = &p.name;
            let ty = &p.ty;
            quote! { pub #name: #ty }
        })
        .collect();

    // Generate function call arguments
    let fn_call_args: Vec<_> = params_info
        .iter()
        .map(|p| {
            let name = &p.name;
            quote! { params.#name }
        })
        .collect();

    // Determine input types and handler logic based on whether function has parameters
    let (input_types, handler_logic) = if params_info.is_empty() {
        // No parameters - function takes no input
        (
            quote! { [] },
            quote! {
                // No parameters - call implementation directly
                let result = #impl_name()
                    .map_err(|e| Error::msg(format!("Function error: {}", e)))?;
            },
        )
    } else {
        // Has parameters - function takes JSON pointer
        (
            quote! { [ValType::I64] },
            quote! {
                // Has parameters - deserialize from JSON
                let input_ptr = inputs[0].unwrap_i64() as u64;

                if input_ptr == 0 {
                    return Err(Error::msg("No input provided"));
                }

                // Get memory handle from offset
                let input_handle = plugin.memory_handle(input_ptr)
                    .ok_or_else(|| Error::msg("Failed to get memory handle"))?;

                // Read JSON string from plugin memory
                let input_str = plugin.memory_str(input_handle)?;

                // Deserialize JSON to our parameter struct
                let params: #param_struct_name = serde_json::from_str(input_str)
                    .map_err(|e| Error::msg(format!("Failed to parse JSON: {}", e)))?;

                // Call the actual implementation function
                let result = #impl_name(#(#fn_call_args),*)
                    .map_err(|e| Error::msg(format!("Function error: {}", e)))?;
            },
        )
    };

    // Original function body
    let fn_body = &input_fn.block;
    let fn_vis = &input_fn.vis; // Visibility is on ItemFn, not Signature
    let fn_output = &input_fn.sig.output;

    let generated = quote! {
        // Implementation function (original logic)
        #fn_vis fn #impl_name(#(#params_info),*) #fn_output #fn_body

        // Parameter struct for JSON deserialization
        #[derive(serde::Deserialize)]
        #[allow(non_camel_case_types)]
        struct #param_struct_name {
            #(#param_fields),*
        }

        // Function wrapper that returns extism::Function
        #[allow(dead_code)]
        pub fn #wrapper_name() -> extism::Function {
            use extism::{Function, UserData, ValType, Val, Error};

            Function::new(
                #fn_name_str,
                #input_types,  // Input types (empty for no params, I64 for params)
                [ValType::I64],  // Output: JSON pointer
                UserData::new(()),
                |plugin, inputs, outputs, _user_data| {
                    // Execute handler logic (either with or without params)
                    #handler_logic

                    // Serialize result to JSON
                    let result_json = serde_json::to_string(&result)
                        .map_err(|e| Error::msg(format!("Failed to serialize result: {}", e)))?;

                    // Allocate memory for result and write JSON
                    let result_handle = plugin.memory_new(result_json.as_bytes())?;

                    // Convert handle to Val for output
                    outputs[0] = plugin.memory_to_val(result_handle);
                    Ok(())
                },
            )
        }

        // Metadata marker for build.rs extraction
        // Format: @VORTIDECK_HOST_METADATA[name="...", description="...", params=[...], return_type="..."]
        #[doc = concat!("@VORTIDECK_HOST_METADATA[name=\"", #fn_name_str, "\", description=\"", #description, "\", return_type=\"", #return_type_str, "\"]")]
        const _: () = ();
    };

    TokenStream::from(generated)
}

// Helper functions

fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &attr.meta {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        if let syn::Lit::Str(lit) = &expr_lit.lit {
                            return Some(lit.value().trim().to_string());
                        }
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct ParamInfo {
    name: syn::Ident,
    ty: syn::Type,
}

impl quote::ToTokens for ParamInfo {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        let ty = &self.ty;
        tokens.extend(quote! { #name: #ty });
    }
}

fn extract_parameters(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Vec<ParamInfo> {
    inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = pat_ident.ident.clone();
                    let ty = (*pat_type.ty).clone();

                    return Some(ParamInfo { name, ty });
                }
            }
            None
        })
        .collect()
}

fn extract_return_type(return_type: &ReturnType) -> String {
    match return_type {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => quote!(#ty).to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
