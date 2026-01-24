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

/// There are two types of transforms:
///
/// - NodeTransform:
///   Transforms that operate directly on a ChildNode. NodeTransforms may mutate,
///   replace or remove the node being processed.
pub trait NodeTransform: Debug {
    fn transform_root(&mut self, node: &mut RootNode) {
        let _ = node;
    }

    fn transform(&mut self, node: &mut TemplateChildNode, context: &mut TransformContext) {
        let _ = node;
        let _ = context;
    }

    fn exit(&mut self, context: &mut TransformContext) {
        let _ = context;
    }

    fn clone_box(&self) -> Box<dyn NodeTransform>;
}

impl Clone for Box<dyn NodeTransform> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

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

#[derive(Debug, Clone)]
pub enum ParentNode {
    Root(*mut RootNode),
    Element(*mut ElementNode),
}

impl ParentNode {
    pub fn children(&self) -> &Vec<TemplateChildNode> {
        unsafe {
            match self {
                Self::Root(node) => &(*(*node)).children,
                Self::Element(node) => (*(*node)).children(),
            }
        }
    }

    pub fn children_mut(&self) -> &mut Vec<TemplateChildNode> {
        unsafe {
            match self {
                Self::Root(node) => &mut (*(*node)).children,
                Self::Element(node) => (*(*node)).children_mut(),
            }
        }
    }
}

pub struct TransformContext<'a> {
    pub ssr: bool,
    pub in_ssr: bool,
    pub node_transforms: Vec<Box<dyn NodeTransform>>,
    pub directive_transforms: HashMap<String, Box<dyn DirectiveTransform>>,

    helpers: ::indexmap::IndexMap<String, usize>,
    pub parent: Option<ParentNode>,
    // we could use a stack but in practice we've only ever needed two layers up
    // so this is more efficient
    grand_parent: Option<ParentNode>,
    pub child_index: usize,
    pub current_node: Option<*mut TransformNode<'a>>,

    node_removed: bool,
    pub global_compile_time_constants: GlobalCompileTimeConstants,
}

impl<'a> TransformContext<'a> {
    fn new(options: TransformOptions) -> Self {
        Self {
            ssr: options.ssr.unwrap_or_default(),
            in_ssr: options.in_ssr.unwrap_or_default(),
            node_transforms: options.node_transforms.unwrap_or_default(),
            directive_transforms: options.directive_transforms.unwrap_or_default(),

            helpers: Default::default(),
            parent: None,
            grand_parent: None,
            child_index: 0,
            current_node: None,

            node_removed: false,
            global_compile_time_constants: options.global_compile_time_constants,
        }
    }

    pub fn helper(&mut self, name: String) -> String {
        let count = self.helpers.get(&name).cloned().unwrap_or_default();
        self.helpers.insert(name.clone(), count + 1);
        name
    }

    pub fn replace_node(&mut self, mut node: TemplateChildNode) {
        if self.global_compile_time_constants.__dev__ {
            if self.current_node.is_none() {
                panic!("Node being replaced is already removed.")
            }
            if self.parent.is_none() {
                panic!("Cannot replace root node.")
            }
        }
        let Some(parent) = &mut self.parent else {
            unreachable!();
        };

        self.current_node = {
            let node_ptr = (&mut node) as *mut _;
            unsafe { Some(&mut TransformNode::TemplateChild(&mut *node_ptr) as *mut _) }
        };
        parent.children_mut()[self.child_index] = node;
    }

    pub fn remove_node(&mut self, node: Option<TemplateChildNode>) {
        /* v8 ignore next 3 */
        if self.global_compile_time_constants.__dev__ && self.parent.is_none() {
            panic!("Cannot remove root node.");
        }
        // const list = context.parent!.children
        // const removalIndex = node
        //   ? list.indexOf(node)
        //   : context.currentNode
        //     ? context.childIndex
        //     : -1
        // if (__DEV__ && removalIndex < 0) {
        //   throw new Error(`node being removed is not a child of current parent`)
        // }
        // // || node == context.currentNode
        if node.is_none() {
            // current node removed
            self.current_node = None;
            self.on_node_removed();
        } else {
            // sibling node removed
            // if (context.childIndex > removalIndex) {
            //   context.childIndex--
            //   context.onNodeRemoved()
            // }
        }
        // context.parent!.children.splice(removalIndex, 1)
    }

    fn on_node_removed(&mut self) {
        self.node_removed = true;
    }

    fn traverse_root_node<'b>(&mut self, mut node: &'b mut RootNode)
    where
        'a: 'b,
    {
        self.current_node = {
            let node_ptr = node as *mut _;
            unsafe { Some(&mut TransformNode::Root(&mut *node_ptr) as *mut _) }
        };

        // apply transform plugins
        let mut node_transforms = self.node_transforms.clone();
        for node_transform in &mut node_transforms {
            node_transform.transform_root(node);
            if let Some(current_node) = self.current_node {
                // node may have been replaced
                node = unsafe {
                    let node = &mut *current_node;
                    let TransformNode::Root(node) = node else {
                        unreachable!();
                    };
                    node
                };
            } else {
                // node was removed
                return;
            }
        }

        let current_node = {
            let node_ptr = node as *mut _;
            unsafe { Some(&mut TransformNode::Root(&mut *node_ptr) as *mut _) }
        };

        let parent = node as *mut _;
        traverse_children(ParentNode::Root(parent), self);

        self.current_node = current_node;
        for node_transform in &mut node_transforms {
            node_transform.exit(self);
        }
    }

    pub fn traverse_node<'b>(&mut self, mut node: &'b mut TemplateChildNode)
    where
        'a: 'b,
    {
        let current_node = {
            let node_ptr = node as *mut _;
            unsafe { Some(&mut TransformNode::TemplateChild(&mut *node_ptr) as *mut _) }
        };
        self.current_node = current_node.clone();
        // apply transform plugins
        let mut node_transforms = self.node_transforms.clone();
        for node_transform in &mut node_transforms {
            node_transform.transform(node, self);
            if let Some(current_node) = self.current_node {
                // node may have been replaced
                node = unsafe {
                    let node = &mut *current_node;
                    let TransformNode::TemplateChild(node) = node else {
                        unreachable!();
                    };
                    node
                };
            } else {
                // node was removed
                return;
            }
        }

        match node {
            TemplateChildNode::Comment(_) => {
                if !self.ssr {
                    // inject import for the Comment symbol, which is needed for creating
                    // comment nodes with `createVNode`
                    self.helper(CreateComment.to_string());
                }
            }
            TemplateChildNode::Interpolation(_) => {
                // no need to traverse, but we need to inject toString helper
                if !self.ssr {
                    self.helper(ToDisplayString.to_string());
                }
            }
            TemplateChildNode::Element(node) => {
                let parent = node as *mut _;
                traverse_children(ParentNode::Element(parent), self);
            }
            _ => {}
        }

        self.current_node = current_node;
        for node_transform in &mut node_transforms {
            node_transform.exit(self);
        }
    }
}

pub fn transform(root: &mut RootNode, options: TransformOptions) {
    let ssr = options.ssr;
    let mut context = TransformContext::new(options);
    context.traverse_root_node(root);

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

fn traverse_children(parent: ParentNode, context: &mut TransformContext) {
    let children = parent.children_mut();
    let mut i = 0;

    context.node_removed = false;
    loop {
        if i >= children.len() {
            break;
        }
        let Some(child) = children.get_mut(i) else {
            unreachable!();
        };

        context.grand_parent = context.parent.take();
        context.parent = Some(parent.clone());
        context.child_index = i;
        context.traverse_node(child);
        if context.node_removed {
            context.node_removed = false;
            i -= 1;
        }
        i += 1;
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

pub trait StructuralDirectiveTransform {
    fn matches(&self, name: &String) -> bool;

    fn transform(&mut self, node: &mut TemplateChildNode) -> Option<Vec<DirectiveNode>> {
        let TemplateChildNode::Element(node) = node else {
            return None;
        };

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
