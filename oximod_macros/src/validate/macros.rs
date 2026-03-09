macro_rules! is_type_safe {
    ($cond:expr, $checks:expr, $field_ident:expr, $msg:expr) => {{
        if !$cond {
            let __msg = $msg;
            let __field_ident = $field_ident;

            $checks.push(::quote::quote_spanned! { __field_ident.span() =>
                compile_error!(#__msg);
            });

            false
        } else {
            true
        }
    }};
}

macro_rules! opt_check {
    ($is_optional:expr, $field_ident:expr, $($body:tt)*) => {{
        let __field_ident = $field_ident;

        if $is_optional {
            ::quote::quote! {
                if let Some(val) = &self.#__field_ident {
                    $($body)*
                }
            }
        } else {
            ::quote::quote! {
                {
                    let val = &self.#__field_ident;
                    $($body)*
                }
            }
        }
    }};
}

pub(crate) use is_type_safe;
pub(crate) use opt_check;
