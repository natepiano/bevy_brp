/// Macro to generate BRP tool implementations
///
/// This macro generates:
/// 1. A unit struct for the tool
/// 2. Implementation of `BrpToolFn` trait that calls `execute_static_brp_call`
/// 3. Implementation of `HasPortField` trait for the params struct
///
/// The parameter struct must be defined separately in params.rs
macro_rules! define_brp_tool {
    ($tool_struct:ident, $params_struct:ident) => {
        pub struct $tool_struct;

        impl crate::tool::BrpToolFn for $tool_struct {
            type Output = crate::brp_tools::handler::BrpMethodResult;

            fn call(
                &self,
                ctx: &crate::tool::HandlerContext<crate::tool::HasPort, crate::tool::HasMethod>,
            ) -> crate::tool::HandlerResponse<Self::Output> {
                Box::pin(crate::brp_tools::handler::execute_static_brp_call::<
                    $params_struct,
                >(ctx))
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
