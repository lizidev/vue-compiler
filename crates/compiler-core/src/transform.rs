use crate::{
    ast::{
        BaseElementProps, DirectiveNode, ElementNode, ElementTypes, JSChildNode, Property,
        RootCodegenNode, RootNode, TemplateChildNode, VNodeCall, VNodeCallChildren,
    },
    options::TransformOptions,
    runtime_helpers::{CreateComment, Fragment, ToDisplayString},
    utils::GlobalCompileTimeConstants,
};
use std::{collections::HashMap, fmt::Debug};
use vue_compiler_shared::PatchFlags;

#[derive(Debug)]
pub enum TransformNode<'a> {
    Root(&'a mut RootNode),
    TemplateChild(&'a mut TemplateChildNode),
}

impl<'a> TransformNode<'a> {
    pub fn children(&self) -> Option<&Vec<TemplateChildNode>> {
        match self {
            Self::Root(node) => Some(&node.children),
            Self::TemplateChild(node) => match node {
                TemplateChildNode::Element(node) => Some(node.children()),
                _ => None,
            },
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<TemplateChildNode>> {
        match self {
            Self::Root(node) => Some(&mut node.children),
            Self::TemplateChild(TemplateChildNode::Element(node)) => Some(node.children_mut()),
            _ => None,
        }
    }
}

/// There are two types of transforms:
///
/// - NodeTransform:
///   Transforms that operate directly on a ChildNode. NodeTransforms may mutate,
///   replace or remove the node being processed.
pub trait NodeTransformState: Debug {
    fn pre_transform(&mut self, parent: &mut TransformNode, context: &mut TransformContext) {
        let _ = parent;
        let _ = context;
    }

    fn transform(&mut self, node: &mut TransformNode, context: &mut TransformContext) {
        let _ = node;
        let _ = context;
    }

    fn pre_exit(&mut self, node: &mut TransformNode, context: &mut TransformContext) {
        let _ = node;
        let _ = context;
    }

    fn exit(&mut self, node: &mut TransformNode, context: &mut TransformContext) {
        let _ = node;
        let _ = context;
    }
}

/// There are two types of transforms:
///
/// - NodeTransform:
///   Transforms that operate directly on a ChildNode. NodeTransforms may mutate,
///   replace or remove the node being processed.
pub type NodeTransform =
    fn(&TransformNode, &mut TransformContext) -> Option<Box<dyn NodeTransformState>>;

pub trait DirectiveTransform: Debug {
    fn transform(
        &mut self,
        dir: &DirectiveNode,
        node: &ElementNode,
        context: &TransformContext,
    ) -> DirectiveTransformResult;

    fn clone_box(&self) -> Box<dyn DirectiveTransform>;
}

impl Clone for Box<dyn DirectiveTransform> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug)]
pub struct DirectiveTransformResult {
    pub props: Vec<Property>,
}

pub struct TransformContext {
    pub ssr: bool,
    pub in_ssr: bool,
    pub node_transforms: Vec<NodeTransform>,
    pub directive_transforms: HashMap<String, Box<dyn DirectiveTransform>>,

    helpers: ::indexmap::IndexMap<String, usize>,

    pub global_compile_time_constants: GlobalCompileTimeConstants,
}

impl TransformContext {
    fn new(options: TransformOptions) -> Self {
        Self {
            ssr: options.ssr.unwrap_or_default(),
            in_ssr: options.in_ssr.unwrap_or_default(),
            node_transforms: options.node_transforms.unwrap_or_default(),
            directive_transforms: options.directive_transforms.unwrap_or_default(),

            helpers: Default::default(),

            global_compile_time_constants: options.global_compile_time_constants,
        }
    }

    pub fn helper(&mut self, name: String) -> String {
        if let Some(count) = self.helpers.get_mut(&name) {
            *count += 1;
        } else {
            self.helpers.insert(name.clone(), 1);
        }
        name
    }

    pub fn remove_helper(&mut self, name: &str) {
        let count = self.helpers.get_mut(name);
        if let Some(count) = count {
            let current_count = *count - 1;
            if current_count == 0 {
                self.helpers.shift_remove(name);
            } else {
                *count = current_count;
            }
        }
    }

    pub fn traverse_node(&mut self, mut node: TransformNode) {
        // apply transform plugins
        let mut node_transforms = self
            .node_transforms
            .clone()
            .into_iter()
            .map_while(|node_transform| node_transform(&node, self))
            .collect::<Vec<_>>();
        for node_transform in &mut node_transforms {
            node_transform.pre_transform(&mut node, self);
        }

        for node_transform in &mut node_transforms {
            node_transform.transform(&mut node, self);
        }

        match &mut node {
            TransformNode::TemplateChild(TemplateChildNode::Comment(_)) => {
                if !self.ssr {
                    // inject import for the Comment symbol, which is needed for creating
                    // comment nodes with `createVNode`
                    self.helper(CreateComment.to_string());
                }
            }
            TransformNode::TemplateChild(TemplateChildNode::Interpolation(_)) => {
                // no need to traverse, but we need to inject toString helper
                if !self.ssr {
                    self.helper(ToDisplayString.to_string());
                }
            }
            TransformNode::TemplateChild(TemplateChildNode::If(node)) => {
                let branchs = node.branches.drain(..).collect::<Vec<_>>();
                for child in branchs {
                    let mut child = TemplateChildNode::IfBranch(child);
                    self.traverse_node(TransformNode::TemplateChild(&mut child));
                    let TemplateChildNode::IfBranch(child) = child else {
                        unreachable!();
                    };
                    node.branches.push(child);
                }
            }
            TransformNode::TemplateChild(TemplateChildNode::IfBranch(node)) => {
                for child in &mut node.children {
                    self.traverse_node(TransformNode::TemplateChild(child));
                }
            }
            TransformNode::TemplateChild(TemplateChildNode::For(node)) => {
                for child in &mut node.children {
                    self.traverse_node(TransformNode::TemplateChild(child));
                }
            }
            TransformNode::TemplateChild(TemplateChildNode::Element(node)) => {
                for child in node.children_mut() {
                    self.traverse_node(TransformNode::TemplateChild(child));
                }
            }
            TransformNode::Root(node) => {
                for child in &mut node.children {
                    self.traverse_node(TransformNode::TemplateChild(child));
                }
            }
            _ => {}
        }

        for node_transform in &mut node_transforms.iter_mut().rev() {
            node_transform.pre_exit(&mut node, self);
        }

        for node_transform in node_transforms.iter_mut().rev() {
            node_transform.exit(&mut node, self);
        }
    }
}

pub fn transform(root: &mut RootNode, options: TransformOptions) {
    let ssr = options.ssr;
    let mut context = TransformContext::new(options);
    context.traverse_node(TransformNode::Root(root));

    if !ssr.unwrap_or_default() {
        create_root_codegen(root, &mut context)
    }
    let TransformContext { helpers, .. } = context;
    root.helpers = helpers.keys().cloned().collect();
    root.transformed = Some(true);
}

fn create_root_codegen<'a>(root: &'a mut RootNode, context: &'a mut TransformContext) {
    if root.children.len() == 1 {
        let single_element_root_child_codegen = get_single_element_root_codegen(root);
        // if the single child is an element, turn it into a block.
        if let Some(single_element_root_child_codegen) = single_element_root_child_codegen {
            // single element root is never hoisted so codegenNode will never be
            // SimpleExpressionNode
            let codegen_node = single_element_root_child_codegen;
            //     if (codegenNode.type === NodeTypes.VNODE_CALL) {
            //       convertToBlock(codegenNode, context)
            //     }
            root.codegen_node = Some(RootCodegenNode::TemplateChild(TemplateChildNode::Element(
                codegen_node,
            )));
        } else {
            // - single <slot/>, IfNode, ForNode: already blocks.
            // - single text node: always patched.
            // root codegen falls through via genNode()
            root.codegen_node = root
                .children
                .first()
                .cloned()
                .map(|n| RootCodegenNode::TemplateChild(n));
        }
    } else if root.children.len() > 1 {
        // root has multiple nodes - return a fragment block.
        let patch_flag = PatchFlags::StableFragment;
        // check if the fragment actually contains a single valid child with
        // the rest being comments
        // if
        //     __DEV__ &&
        //     children.filter(c => c.type !== NodeTypes.COMMENT).length === 1
        // {
        // patch_flag |= PatchFlags::DevRootFragment;
        // }
        let tag = context.helper(Fragment.to_string());
        root.codegen_node = Some(RootCodegenNode::JSChild(JSChildNode::VNodeCall(
            VNodeCall::new(
                Some(context),
                tag,
                None,
                Some(VNodeCallChildren::TemplateChildNodeList(
                    root.children.clone(),
                )),
                Some(patch_flag),
                Some(true),
                None,
                /* isComponent */
                Some(false),
                None,
            ),
        )));
    } else {
        // no children = noop. codegen will return null.
    }
}

fn get_single_element_root_codegen(root: &RootNode) -> Option<ElementNode> {
    let children = root
        .children
        .iter()
        .filter(|x| !matches!(x, TemplateChildNode::Comment(_)));
    if children.count() == 1 {
        let mut children = root
            .children
            .iter()
            .filter(|x| !matches!(x, TemplateChildNode::Comment(_)));
        // TODO
        // !isSlotOutlet(children[0])
        if let Some(node) = children.next()
            && let TemplateChildNode::Element(node) = node
            && !matches!(node.tag_type(), ElementTypes::Slot)
        {
            return Some(node.clone());
        }
    }

    None
}

// A structural directive transform is technically also a NodeTransform;
// Only v-if and v-for fall into this category.
pub trait StructuralDirectiveTransform {
    fn matches(&self, name: &String) -> bool;

    fn transform(&mut self, node: &mut ElementNode) -> Option<Vec<DirectiveNode>> {
        let props: Vec<_> = node.props_mut().drain(..).collect();
        let mut dirs: Vec<DirectiveNode> = Vec::new();

        for prop in props {
            match prop {
                BaseElementProps::Directive(prop) if self.matches(&prop.name) => {
                    dirs.push(prop);
                }
                _ => {
                    node.props_mut().push(prop);
                }
            }
        }

        if dirs.is_empty() {
            return None;
        }

        Some(dirs)
    }
}
