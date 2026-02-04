macro_rules! symbol {
    (pub struct $StructName:ident : $lit:literal) => {
        pub struct $StructName;

        impl std::fmt::Display for $StructName {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $lit)
            }
        }
    };
}

symbol!(pub struct Fragment: "Fragment");
symbol!(pub struct Teleport: "Teleport");
symbol!(pub struct Suspense: "Suspense");

symbol!(pub struct OpenBlock: "openBlock");
symbol!(pub struct CreateBlock: "createBlock");
symbol!(pub struct CreateElementBlock: "createElementBlock");
symbol!(pub struct CreateVNode: "createVNode");
symbol!(pub struct CreateElementVNode: "createElementVNode");
symbol!(pub struct CreateComment: "createCommentVNode");
symbol!(pub struct CreateText: "createTextVNode");
symbol!(pub struct CreateStatic: "createStaticVNode");
symbol!(pub struct ResolveComponent: "resolveComponent");
symbol!(pub struct ResolveDirective: "resolveDirective");

symbol!(pub struct RenderList: "renderList");

symbol!(pub struct ToDisplayString: "toDisplayString");
symbol!(pub struct NormalizeClass: "normalizeClass");

symbol!(pub struct SetBlockTracking: "setBlockTracking");
