macro_rules! opt_check {
    (
        $is_opt:expr,
        $field:ident,
        $($check:tt)*
    ) => {
        {
        if $is_opt {
            quote! {
                if let Some(val) = &self.#$field {
                    $($check)*
                }
            }
        } else {
            quote! {
                {
                    let val = &self.#$field;
                    $($check)*
                }
            }
        }
        }
    };
}
pub(crate) use opt_check;

macro_rules! is_type_safe {
    ($cond:expr, $checks:expr, $field_ident:expr, $msg:expr) => {{
        if !$cond {
            let __msg = $msg;

            $checks.push(quote_spanned! { $field_ident.span() =>
                compile_error!(#__msg);
            });

            false
        } else {
            true
        }
    }};
}
pub(crate) use is_type_safe;
