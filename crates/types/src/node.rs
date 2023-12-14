use hirola::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::{Rc, Weak};
use std::{fmt, mem};

/// Rendering backend for Server Side Rendering, aka. SSR.
/// Offers interior mutability and is not thread safe.
#[derive(Debug, Clone)]
enum NodeType {
    Element(RefCell<Element>),
    Comment(RefCell<Comment>),
    Text(RefCell<Text>),
    Fragment(RefCell<Fragment>),
}

#[derive(Debug, Clone)]
pub struct NodeInner {
    ty: Rc<NodeType>,
    /// No parent if `Weak::upgrade` returns `None`.
    parent: RefCell<Weak<NodeInner>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Node(#[serde(with = "node_type_serde")] Rc<NodeInner>);

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0.ty, &other.0.ty)
    }
}

impl Eq for Node {}

impl Node {
    fn new(ty: NodeType) -> Self {
        Self(Rc::new(NodeInner {
            ty: Rc::new(ty),
            parent: RefCell::new(Weak::new()), // no parent
        }))
    }

    fn set_parent(&self, parent: Weak<NodeInner>) {
        if let Some(old_parent) = self.parent_node() {
            old_parent.try_remove_child(self);
        }

        *self.0.parent.borrow_mut() = parent;
    }

    #[track_caller]
    fn unwrap_element(&self) -> &RefCell<Element> {
        match self.0.ty.as_ref() {
            NodeType::Element(e) => e,
            _ => panic!("node is not an element"),
        }
    }

    #[track_caller]
    fn unwrap_text(&self) -> &RefCell<Text> {
        match &self.0.ty.as_ref() {
            NodeType::Text(e) => e,
            _ => panic!("node is not a text node"),
        }
    }

    // FIXME: recursively visit Fragments and call try_remove_child
    fn try_remove_child(&self, child: &Self) {
        let mut children = match self.0.ty.as_ref() {
            NodeType::Element(e) => mem::take(&mut e.borrow_mut().children.0),
            NodeType::Fragment(f) => mem::take(&mut f.borrow_mut().0),
            _ => panic!("node type cannot have children"),
        };

        if let Some(index) = children
            .iter()
            .enumerate()
            .find_map(|(i, c)| (c == child).then_some(i))
        {
            children.remove(index);
        } else {
            // try remove from child Fragments
            for c in &children {
                if let NodeType::Fragment(fragment) = c.0.ty.as_ref() {
                    for c in &fragment.borrow().0 {
                        c.try_remove_child(child);
                    }
                }
            }
        }

        match self.0.ty.as_ref() {
            NodeType::Element(e) => e.borrow_mut().children.0 = children,
            NodeType::Fragment(f) => f.borrow_mut().0 = children,
            _ => panic!("node type cannot have children"),
        };
    }
}

impl GenericNode for Node {
    fn element(tag: &str) -> Self {
        Node::new(NodeType::Element(RefCell::new(Element {
            name: tag.to_string(),
            attributes: HashMap::new(),
            children: Default::default(),
        })))
    }

    fn text_node(text: &str) -> Self {
        Node::new(NodeType::Text(RefCell::new(Text(text.to_string()))))
    }

    fn fragment() -> Self {
        Node::new(NodeType::Fragment(Default::default()))
    }

    fn marker() -> Self {
        Node::new(NodeType::Comment(Default::default()))
    }

    fn set_attribute(&self, name: &str, value: &str) {
        self.unwrap_element()
            .borrow_mut()
            .attributes
            .insert(name.to_string(), value.to_string());
    }

    fn append_child(&self, child: &Self) {
        child.set_parent(Rc::downgrade(&self.0));

        match self.0.ty.as_ref() {
            NodeType::Element(element) => element.borrow_mut().children.0.push(child.clone()),
            NodeType::Fragment(fragment) => fragment.borrow_mut().0.push(child.clone()),
            _ => panic!("node type cannot have children"),
        }
    }

    fn insert_child_before(&self, new_node: &Self, reference_node: Option<&Self>) {
        if let Some(reference_node) = reference_node {
            debug_assert_eq!(
                reference_node.parent_node().as_ref(),
                Some(self),
                "reference node is not a child of this node"
            );
        }

        new_node.set_parent(Rc::downgrade(&self.0));

        let mut children = match self.0.ty.as_ref() {
            NodeType::Element(e) => mem::take(&mut e.borrow_mut().children.0),
            NodeType::Fragment(f) => mem::take(&mut f.borrow_mut().0),
            _ => panic!("node type cannot have children"),
        };

        match reference_node {
            None => self.append_child(new_node),
            Some(reference) => {
                children.insert(
                    children
                        .iter()
                        .enumerate()
                        .find_map(|(i, child)| (child == reference).then_some(i))
                        .expect("couldn't find reference node"),
                    new_node.clone(),
                );
            }
        }

        match self.0.ty.as_ref() {
            NodeType::Element(e) => e.borrow_mut().children.0 = children,
            NodeType::Fragment(f) => f.borrow_mut().0 = children,
            _ => panic!("node type cannot have children"),
        };
    }

    fn remove_child(&self, child: &Self) {
        let mut children = match self.0.ty.as_ref() {
            NodeType::Element(e) => mem::take(&mut e.borrow_mut().children.0),
            NodeType::Fragment(f) => mem::take(&mut f.borrow_mut().0),
            _ => panic!("node type cannot have children"),
        };

        let index = children
            .iter()
            .enumerate()
            .find_map(|(i, c)| (c == child).then_some(i))
            .expect("couldn't find child");
        children.remove(index);

        match self.0.ty.as_ref() {
            NodeType::Element(e) => e.borrow_mut().children.0 = children,
            NodeType::Fragment(f) => f.borrow_mut().0 = children,
            _ => panic!("node type cannot have children"),
        };
    }

    fn replace_child(&self, old: &Self, new: &Self) {
        new.set_parent(Rc::downgrade(&self.0));

        let mut ele = self.unwrap_element().borrow_mut();
        let children = &mut ele.children.0;
        let index = children
            .iter()
            .enumerate()
            .find_map(|(i, c)| (c == old).then_some(i))
            .expect("Couldn't find child");
        children[index] = new.clone();
    }

    fn insert_sibling_before(&self, child: &Self) {
        child.set_parent(Rc::downgrade(
            &self.parent_node().expect("no parent for this node").0,
        ));

        self.parent_node()
            .unwrap()
            .insert_child_before(child, Some(self));
    }

    fn parent_node(&self) -> Option<Self> {
        self.0.parent.borrow().upgrade().map(Node)
    }

    fn next_sibling(&self) -> Option<Self> {
        unimplemented!()
    }

    fn remove_self(&self) {
        unimplemented!()
    }

    fn children(&self) -> RefCell<Vec<Node>> {
        unimplemented!()
    }

    fn update_inner_text(&self, text: &str) {
        self.unwrap_text().borrow_mut().0 = text.to_string();
    }

    fn replace_children_with(&self, _node: &Self) {
        unimplemented!()
    }
    fn effect(&self, _future: impl std::future::Future<Output = ()> + 'static) {
        // panic!("Node does not support effects, please use NodeAsync!")
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.ty.as_ref() {
            NodeType::Element(x) => write!(f, "{}", x.borrow()),
            NodeType::Comment(x) => write!(f, "{}", x.borrow()),
            NodeType::Text(x) => write!(f, "{}", x.borrow()),
            NodeType::Fragment(x) => write!(f, "{}", x.borrow()),
        }
    }
}

impl Render<Node> for Node {
    fn render_into(self: Box<Self>, parent: &Node) -> Result<(), Error> {
        parent.append_child(&self);
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Element {
    name: String,
    attributes: HashMap<String, String>,
    children: Fragment,
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}", self.name)?;
        for (name, value) in &self.attributes {
            write!(f, r#" {}="{}""#, name, &value)?;
        }
        write!(f, ">{}</{}>", self.children, self.name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct Comment(String);

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<!--{}-->", self.0.replace("-->", "--&gt;"))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct Text(String);

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct Fragment(Vec<Node>);

impl fmt::Display for Fragment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for child in &self.0 {
            write!(f, "{}", child)?;
        }
        Ok(())
    }
}

mod node_type_serde {
    use serde::de::Deserializer;
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};

    use super::{Comment, Element, Fragment, NodeInner, NodeType, Text};
    #[derive(Deserialize, Serialize)]
    enum CustomNodeType {
        Element(Element),
        Comment(Comment),
        Text(Text),
        Fragment(Fragment),
    }

    pub fn serialize<S>(val: &Rc<NodeInner>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &val.ty.as_ref() {
            NodeType::Element(t) => Element::serialize(&t.borrow(), s),
            NodeType::Comment(t) => Comment::serialize(&t.borrow(), s),
            NodeType::Text(t) => Text::serialize(&t.borrow(), s),
            NodeType::Fragment(t) => Fragment::serialize(&t.borrow(), s),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Rc<NodeInner>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ty = CustomNodeType::deserialize(d)
            .map(|res| match res {
                CustomNodeType::Element(e) => NodeType::Element(RefCell::new(e)),
                CustomNodeType::Comment(e) => NodeType::Comment(RefCell::new(e)),
                CustomNodeType::Text(e) => NodeType::Text(RefCell::new(e)),
                CustomNodeType::Fragment(e) => NodeType::Fragment(RefCell::new(e)),
            })?
            .into();
        Ok(Rc::new(NodeInner {
            ty,
            parent: RefCell::new(Weak::new()),
        }))
    }
}
