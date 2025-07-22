/// Macro to generate BRP tool implementations
///
/// This macro generates:
/// 1. A unit struct for the tool
/// 2. Implementation of `UnifiedToolFn` trait that calls `execute_static_brp_call`
/// 3. Implementation of `HasPortField` trait for the params struct
/// 4. Implementation of `HasBrpMethod` trait with the specified BRP method
///
/// Usage: `define_brp_tool!`(`ToolStruct`, `ParamsStruct`, `BRP_METHOD_CONSTANT`);
/// The parameter struct must be defined separately in params.rs
macro_rules! define_brp_tool {
    ($tool_struct:ident, $params_struct:ident, $brp_method:expr) => {
        pub struct $tool_struct;

        impl crate::tool::UnifiedToolFn for $tool_struct {
            type Output = crate::brp_tools::handler::BrpMethodResult;
            type CallInfoData = crate::response::BrpCallInfo;

            fn call(
                &self,
                ctx: &crate::tool::HandlerContext,
            ) -> crate::tool::HandlerResponse<(Self::CallInfoData, Self::Output)> {
                let ctx_clone = ctx.clone();
                Box::pin(async move {
                    let params = ctx_clone.extract_typed_params::<$params_struct>()?;
                    let port =
                        <$params_struct as crate::brp_tools::handler::HasPortField>::port(&params);
                    let result = crate::brp_tools::handler::execute_static_brp_call::<
                        $tool_struct,
                        $params_struct,
                    >(&ctx_clone)
                    .await?;

                    Ok((
                        crate::response::BrpCallInfo {
                            method: <$tool_struct as crate::tool::HasBrpMethod>::brp_method(),
                            port,
                        },
                        result,
                    ))
                })
            }
        }

        impl crate::tool::HasBrpMethod for $tool_struct {
            fn brp_method() -> &'static str {
                $brp_method
            }
        }

        impl crate::brp_tools::handler::HasPortField for super::params::$params_struct {
            fn port(&self) -> u16 {
                self.port
            }
        }
    };
}

pub(super) use define_brp_tool;
