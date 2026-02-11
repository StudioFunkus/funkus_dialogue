//! # Core dialogue graph structure.
//!
//! `DialogueGraph` manages dialogue nodes and their connections while handing out stable
//! [`NodeId`] handles for callers to use. Each call to [`DialogueGraph::add_node`] returns
//! the identifier needed to connect nodes, serialize assets, or drive the runtime, and
//! those identifiers remain valid even after other nodes are removed.
//!
//! ```rust
//! use funkus_dialogue_core::graph::{ConnectionData, DialogueGraph, DialogueNode};
//!
//! let mut graph = DialogueGraph::new().with_name("Greeting");
//! let start = graph.add_node(DialogueNode::text("Hello!").with_speaker("Guide"));
//! let choice = graph
//!     .add_node(DialogueNode::choice().with_prompt("How do you respond?").unwrap());
//! graph.connect(start, choice, ConnectionData::new(None)).unwrap();
//! graph.set_start_node(start).unwrap();
//! ```

use bevy::prelude::*;
use petgraph::Direction;
use petgraph::algo;
use petgraph::stable_graph::{NodeIndex as StableNodeIndex, StableDiGraph};
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::ConnectionData;
use super::node::NodeId;
use super::nodes::DialogueNode;
use crate::DialogueValue;
use crate::registry::{DialogueEffect, DialogueMessageCall};

/// Represents a complete dialogue graph with nodes, connections, and metadata.
///
/// `DialogueGraph` wraps petgraph's [`StableDiGraph`], issuing stable [`NodeId`]
/// handles whenever [`DialogueGraph::add_node`] is called. Those handles are used
/// throughout the API for connecting nodes, marking the start node, and driving the
/// runtime layer.
///
/// # Fields
///
/// - `graph`: internal storage for [`DialogueNode`] values and [`ConnectionData`] edges
/// - `start_node`: optional entry point for the dialogue
/// - `name`: optional label for tooling or display purposes
#[derive(Debug, Clone, Default, Reflect)]
pub struct DialogueGraph {
    /// The underlying stable directed graph storing nodes and edges.
    #[reflect(ignore)]
    graph: StableDiGraph<DialogueNode, ConnectionData>,
    /// Optional starting node for this dialogue graph.
    pub start_node: Option<NodeId>,
    /// Optional display name for the dialogue.
    pub name: Option<String>,
}

impl Serialize for DialogueGraph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct SerialNode {
            #[serde(rename = "type")]
            node_type: &'static str,
            id: NodeId,
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            prompt: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            presentation_key: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            speaker: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            portrait: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            effect: Option<DialogueEffect>,
            #[serde(skip_serializing_if = "Option::is_none")]
            message: Option<DialogueMessageCall>,
        }

        #[derive(Serialize)]
        struct SerialConnection {
            from: NodeId,
            to: NodeId,
            label: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            choice_key: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            order: Option<u32>,
        }

        #[derive(Serialize)]
        struct SerialGraph {
            nodes: Vec<SerialNode>,
            connections: Vec<SerialConnection>,
            start_node: Option<NodeId>,
            name: Option<String>,
        }

        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        for index in self.graph.node_indices() {
            if let Some(node) = self.graph.node_weight(index) {
                let node_id = NodeId::from_index(index);
                let (node_type, text, prompt, presentation_key, effect, message) = match node {
                    DialogueNode::Text { text, .. } => {
                        ("Text", Some(text.clone()), None, None, None, None)
                    }
                    DialogueNode::Choice {
                        prompt,
                        presentation_key,
                        ..
                    } => (
                        "Choice",
                        None,
                        prompt.clone(),
                        presentation_key.clone(),
                        None,
                        None,
                    ),
                    DialogueNode::Effect { effect } => {
                        ("Effect", None, None, None, Some(effect.clone()), None)
                    }
                    DialogueNode::Message { message } => {
                        ("Message", None, None, None, None, Some(message.clone()))
                    }
                };
                let (speaker, portrait) = match node {
                    DialogueNode::Text {
                        speaker, portrait, ..
                    }
                    | DialogueNode::Choice {
                        speaker, portrait, ..
                    } => (speaker.clone(), portrait.clone()),
                    DialogueNode::Effect { .. } | DialogueNode::Message { .. } => (None, None),
                };

                nodes.push(SerialNode {
                    node_type,
                    id: node_id,
                    text,
                    prompt,
                    presentation_key,
                    speaker,
                    portrait,
                    effect,
                    message,
                });
            }
        }

        for edge in self.graph.edge_indices() {
            if let Some((from_idx, to_idx)) = self.graph.edge_endpoints(edge) {
                let from = NodeId::from_index(from_idx);
                let to = NodeId::from_index(to_idx);
                let (label, choice_key, order) = self
                    .graph
                    .edge_weight(edge)
                    .map_or((None, None, None), |data| {
                        (data.label.clone(), data.choice_key.clone(), data.order)
                    });
                connections.push(SerialConnection {
                    from,
                    to,
                    label,
                    choice_key,
                    order,
                });
            }
        }

        nodes.sort_by_key(|node| node.id.raw());
        connections.sort_by(|a, b| {
            a.from
                .raw()
                .cmp(&b.from.raw())
                .then(a.to.raw().cmp(&b.to.raw()))
        });

        SerialGraph {
            nodes,
            connections,
            start_node: self.start_node,
            name: self.name.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DialogueGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SerialNode {
            #[serde(rename = "type")]
            node_type: String,
            id: NodeId,
            text: Option<String>,
            prompt: Option<String>,
            #[serde(default)]
            presentation_key: Option<String>,
            speaker: Option<String>,
            portrait: Option<String>,
            effect: Option<DialogueEffect>,
            message: Option<DialogueMessageCall>,
        }

        #[derive(Deserialize)]
        struct SerialConnection {
            from: NodeId,
            to: NodeId,
            label: Option<String>,
            #[serde(default)]
            choice_key: Option<String>,
            #[serde(default)]
            order: Option<u32>,
        }

        #[derive(Deserialize)]
        struct SerialGraph {
            nodes: Vec<SerialNode>,
            connections: Vec<SerialConnection>,
            start_node: Option<NodeId>,
            name: Option<String>,
        }

        let data = SerialGraph::deserialize(deserializer)?;

        let mut graph = DialogueGraph::new();
        graph.name = data.name;

        let mut id_map: HashMap<NodeId, NodeId> = HashMap::new();

        for node_data in data.nodes {
            let mut node = match node_data.node_type.as_str() {
                "Text" => DialogueNode::text(node_data.text.unwrap_or_default()),
                "Choice" => {
                    let mut choice = DialogueNode::choice();
                    if let Some(prompt) = node_data.prompt {
                        let _ = choice.set_prompt(prompt);
                    }
                    if let Some(presentation_key) = node_data.presentation_key {
                        let _ = choice.set_presentation_key(presentation_key);
                    }
                    choice
                }
                "Effect" => {
                    let effect = node_data.effect.unwrap_or_else(|| {
                        DialogueEffect::set("game.flag", DialogueValue::Bool(true))
                    });
                    DialogueNode::effect(effect)
                }
                "Message" => {
                    let message = node_data
                        .message
                        .unwrap_or_else(|| DialogueMessageCall::new("game.message"));
                    DialogueNode::message(message)
                }
                // Ignore unknown variants so assets remain forward-compatible
                _ => continue,
            };

            if let Some(speaker) = node_data.speaker {
                node.set_speaker(speaker);
            }

            if let Some(portrait) = node_data.portrait {
                node.set_portrait(portrait);
            }

            let assigned_id = graph.add_node(node);
            id_map.insert(node_data.id, assigned_id);
        }

        if let Some(start_serial) = data.start_node {
            if let Some(mapped) = id_map.get(&start_serial) {
                graph.start_node = Some(*mapped);
            }
        }

        for conn in data.connections {
            if let (Some(&from), Some(&to)) = (id_map.get(&conn.from), id_map.get(&conn.to)) {
                let data = ConnectionData::new(conn.label.clone());
                let data = if let Some(order) = conn.order {
                    data.with_order(order)
                } else {
                    data
                };
                let data = if let Some(choice_key) = conn.choice_key {
                    data.with_choice_key(choice_key)
                } else {
                    data
                };
                let _ = graph.connect(from, to, data);
            }
        }

        Ok(graph)
    }
}

impl DialogueGraph {
    /// Creates a new, empty dialogue graph with no nodes or start point.
    ///
    /// Typically you call [`DialogueGraph::add_node`] immediately afterward to seed the graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns a display name to the dialogue graph using builder syntax.
    ///
    /// This is a convenience helper for chaining with other builder-style methods when
    /// constructing graphs inline in tests or examples.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    fn node_index(&self, id: NodeId) -> Option<StableNodeIndex> {
        let idx = id.into_index();
        if self.graph.contains_node(idx) {
            Some(idx)
        } else {
            None
        }
    }

    fn require_index(&self, id: NodeId) -> Result<StableNodeIndex, String> {
        self.node_index(id)
            .ok_or_else(|| format!("Node {:?} not found", id))
    }

    /// Adds a node to the graph and returns its stable identifier.
    ///
    /// The returned [`NodeId`] should be retained by the caller and used when wiring
    /// connections or updating the node later.
    pub fn add_node(&mut self, node: DialogueNode) -> NodeId {
        let index = self.graph.add_node(node);
        NodeId::from_index(index)
    }

    /// Retrieves a node by its identifier.
    ///
    /// Returns `None` if the handle is stale or the node has been removed.
    /// This is typically used when presenting dialogue content at runtime.
    pub fn get_node(&self, id: NodeId) -> Option<&DialogueNode> {
        self.node_index(id)
            .and_then(|index| self.graph.node_weight(index))
    }

    /// Retrieves a mutable reference to a node by its identifier.
    ///
    /// When the identifier no longer refers to an active node the method returns `None`.
    /// Useful for tooling that edits an existing node in-place.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut DialogueNode> {
        let idx = self.node_index(id)?;
        self.graph.node_weight_mut(idx)
    }

    /// Marks the node as the starting point for this dialogue.
    ///
    /// An error is returned if `id` does not refer to an active node.
    pub fn set_start_node(&mut self, id: NodeId) -> Result<(), String> {
        self.require_index(id)?;
        self.start_node = Some(id);
        Ok(())
    }

    /// Clears the currently assigned start node, if any.
    pub fn clear_start_node(&mut self) {
        self.start_node = None;
    }

    /// Returns the starting node, if it exists.
    ///
    /// This is primarily helpful for editor tooling that wants to preview the
    /// current entry point without walking the graph.
    pub fn get_start_node(&self) -> Option<&DialogueNode> {
        self.start_node.and_then(|id| self.get_node(id))
    }

    /// Validates the graph structure.
    ///
    /// Ensures that the start node exists and that every node is reachable from it.
    pub fn validate(&self) -> Result<(), String> {
        let start_id = self
            .start_node
            .ok_or_else(|| "Start node is not set".to_string())?;
        let start_index = self.require_index(start_id)?;

        // Ensure all edges reference existing nodes.
        for edge in self.graph.edge_indices() {
            if let Some((_, to_idx)) = self.graph.edge_endpoints(edge) {
                if !self.graph.contains_node(to_idx) {
                    return Err("Graph contains a connection to a removed node".to_string());
                }
            }
        }

        for (idx, _) in self.graph.node_references() {
            if idx == start_index {
                continue;
            }
            if !algo::has_path_connecting(&self.graph, start_index, idx, None) {
                return Err(format!(
                    "Node {:?} is unreachable from the start node",
                    NodeId::from_index(idx)
                ));
            }
        }

        self.validate_choice_connection_metadata()?;

        Ok(())
    }

    fn validate_choice_connection_metadata(&self) -> Result<(), String> {
        for (idx, node) in self.graph.node_references() {
            if !matches!(node, DialogueNode::Choice { .. }) {
                continue;
            }

            let node_id = NodeId::from_index(idx);
            let mut seen: HashSet<String> = HashSet::new();
            for (_, data) in self.get_connections(node_id) {
                let Some(choice_key) = data.choice_key.as_deref() else {
                    continue;
                };

                let trimmed = choice_key.trim();
                if trimmed.is_empty() {
                    return Err(format!("Choice node {:?} has an empty choice_key", node_id));
                }

                if !seen.insert(trimmed.to_string()) {
                    return Err(format!(
                        "Choice node {:?} has duplicate choice_key '{}'",
                        node_id, trimmed
                    ));
                }
            }
        }

        Ok(())
    }

    /// Returns all outward connections from the supplied node in stable order.
    ///
    /// The labels in the tuple are cloned from the underlying [`ConnectionData`] for convenience.
    /// Connections are sorted by their explicit `order` field (ascending), falling back to node id
    /// for ties. If you need a different ordering, sort the results yourself.
    pub fn get_connected_nodes(&self, id: NodeId) -> Vec<(NodeId, Option<String>)> {
        self.get_outgoing_connections(id)
            .into_iter()
            .map(|(target, data)| (target, data.label))
            .collect()
    }

    /// Returns all outward connections from the supplied node in stable order with full metadata.
    pub fn get_outgoing_connections(&self, id: NodeId) -> Vec<(NodeId, ConnectionData)> {
        let mut connections = self.get_connections(id);
        connections.sort_by(|(a_id, a_data), (b_id, b_data)| {
            let a_order = a_data.order.unwrap_or(u32::MAX);
            let b_order = b_data.order.unwrap_or(u32::MAX);
            a_order
                .cmp(&b_order)
                .then_with(|| a_id.raw().cmp(&b_id.raw()))
        });
        connections
            .into_iter()
            .map(|(target, data)| (target, data.clone()))
            .collect()
    }

    /// Returns the number of active nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns a collection of all node identifiers.
    ///
    /// The order matches the internal storage order and can change when nodes are removed.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.graph.node_indices().map(NodeId::from_index).collect()
    }

    /// Returns an iterator over all nodes in the graph.
    pub fn nodes_iter(&self) -> impl Iterator<Item = &DialogueNode> {
        self.graph.node_weights()
    }

    /// Checks whether the supplied identifier refers to an active node.
    ///
    /// This is a lightweight way to guard editor actions before attempting a mutation.
    pub fn contains_node(&self, id: NodeId) -> bool {
        self.node_index(id).is_some()
    }

    /// Replaces the contents of an existing node.
    ///
    /// Returns an error if the handle points to a node that no longer exists.
    /// Consumers can use this to swap in edited dialogue data without reallocating IDs.
    pub fn update_node(&mut self, id: NodeId, node: DialogueNode) -> Result<(), String> {
        let idx = self.require_index(id)?;
        if let Some(existing) = self.graph.node_weight_mut(idx) {
            *existing = node;
            Ok(())
        } else {
            Err(format!("Node {:?} not found", id))
        }
    }

    /// Removes a node from the graph along with its connections.
    ///
    /// The method clears the start node automatically if the removed node was the entry point.
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), String> {
        let idx = self.require_index(id)?;
        if self.graph.remove_node(idx).is_some() {
            if self.start_node == Some(id) {
                self.start_node = None;
            }
            Ok(())
        } else {
            Err(format!("Node {:?} not found", id))
        }
    }

    /// Creates a connection between two nodes using the provided edge data.
    ///
    /// Errors are returned when either endpoint is missing, keeping petgraph's indices
    /// out of the public API.
    pub fn connect(
        &mut self,
        from: NodeId,
        to: NodeId,
        data: ConnectionData,
    ) -> Result<(), String> {
        let from_idx = self.require_index(from)?;
        let to_idx = self.require_index(to)?;
        let mut data = data;
        if data.order.is_none() {
            data.order = Some(self.next_connection_order(from_idx));
        }
        self.graph.add_edge(from_idx, to_idx, data);
        Ok(())
    }

    /// Removes a connection between two nodes.
    ///
    /// If no edge existed, an error is reported so tooling can surface the failure.
    pub fn disconnect(&mut self, from: NodeId, to: NodeId) -> Result<(), String> {
        let from_idx = self.require_index(from)?;
        let to_idx = self.require_index(to)?;

        let mut removed = false;
        while let Some(edge_id) = self.graph.find_edge(from_idx, to_idx) {
            self.graph.remove_edge(edge_id);
            removed = true;
        }

        if removed {
            Ok(())
        } else {
            Err(format!("No connection from {:?} to {:?}", from, to))
        }
    }

    /// Updates the label for a connection between two nodes.
    ///
    /// Returns an error if no connection exists.
    pub fn update_connection_label(
        &mut self,
        from: NodeId,
        to: NodeId,
        label: Option<String>,
    ) -> Result<(), String> {
        let from_idx = self.require_index(from)?;
        let to_idx = self.require_index(to)?;

        if let Some(edge_id) = self.graph.find_edge(from_idx, to_idx) {
            if let Some(data) = self.graph.edge_weight_mut(edge_id) {
                data.label = label;
                return Ok(());
            }
        }

        Err(format!("No connection from {:?} to {:?}", from, to))
    }

    /// Updates the semantic key for a connection between two nodes.
    ///
    /// Returns an error if no connection exists.
    pub fn update_connection_choice_key(
        &mut self,
        from: NodeId,
        to: NodeId,
        choice_key: Option<String>,
    ) -> Result<(), String> {
        let from_idx = self.require_index(from)?;
        let to_idx = self.require_index(to)?;

        if let Some(edge_id) = self.graph.find_edge(from_idx, to_idx) {
            if let Some(data) = self.graph.edge_weight_mut(edge_id) {
                data.choice_key = choice_key;
                return Ok(());
            }
        }

        Err(format!("No connection from {:?} to {:?}", from, to))
    }

    /// Reorders outgoing connections for a node using the provided target ordering.
    ///
    /// Every outgoing connection from `from` must be represented exactly once in
    /// `ordered_targets`. The supplied order is stored in each connection's
    /// [`ConnectionData::order`] field.
    pub fn reorder_connections(
        &mut self,
        from: NodeId,
        ordered_targets: &[NodeId],
    ) -> Result<(), String> {
        let from_idx = self.require_index(from)?;
        let existing: Vec<NodeId> = self
            .graph
            .edges_directed(from_idx, Direction::Outgoing)
            .filter_map(|edge| self.graph.node_weight(edge.target()).map(|_| edge.target()))
            .map(NodeId::from_index)
            .collect();

        if existing.len() != ordered_targets.len() {
            return Err(format!(
                "Expected {} outgoing connections for {:?}, got {}",
                existing.len(),
                from,
                ordered_targets.len()
            ));
        }

        let mut seen = std::collections::HashSet::new();
        for target in ordered_targets {
            if !seen.insert(*target) {
                return Err(format!(
                    "Duplicate target {:?} provided when reordering connections",
                    target
                ));
            }
        }

        for target in &existing {
            if !seen.contains(target) {
                return Err(format!(
                    "Missing target {:?} when reordering connections for {:?}",
                    target, from
                ));
            }
        }

        for (index, target) in ordered_targets.iter().enumerate() {
            let to_idx = self.require_index(*target)?;
            let Some(edge_id) = self.graph.find_edge(from_idx, to_idx) else {
                return Err(format!("No connection from {:?} to {:?}", from, target));
            };
            if let Some(data) = self.graph.edge_weight_mut(edge_id) {
                data.order = Some(index as u32);
            }
        }

        Ok(())
    }

    /// Retrieves all connections leaving a node, including their edge data.
    ///
    /// The returned vector borrows [`ConnectionData`] weights; callers that need owned labels
    /// can use [`DialogueGraph::get_connected_nodes`] instead.
    pub fn get_connections(&self, from: NodeId) -> Vec<(NodeId, &ConnectionData)> {
        let mut results = Vec::new();
        if let Some(index) = self.node_index(from) {
            for edge in self.graph.edges_directed(index, Direction::Outgoing) {
                let target_idx = edge.target();
                if self.graph.contains_node(target_idx) {
                    results.push((NodeId::from_index(target_idx), edge.weight()));
                }
            }
        }
        results
    }

    fn next_connection_order(&self, from: StableNodeIndex) -> u32 {
        let mut max_order: Option<u32> = None;
        for edge in self.graph.edges_directed(from, Direction::Outgoing) {
            if let Some(order) = edge.weight().order {
                max_order = Some(match max_order {
                    Some(current) => current.max(order),
                    None => order,
                });
            }
        }
        max_order.map_or(0, |value| value.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn build_sample_graph() -> (DialogueGraph, NodeId, NodeId, NodeId, NodeId) {
        let mut graph = DialogueGraph::new().with_name("Sample");
        let start = graph.add_node(DialogueNode::text("Start").with_speaker("Guide"));
        let choice = graph.add_node(
            DialogueNode::choice()
                .with_prompt("Pick one")
                .unwrap()
                .with_presentation_key("inline_badges")
                .unwrap(),
        );
        let branch_a = graph.add_node(DialogueNode::text("A"));
        let branch_b = graph.add_node(DialogueNode::text("B"));

        graph
            .connect(start, choice, ConnectionData::new(None))
            .unwrap();
        graph
            .connect(
                choice,
                branch_a,
                ConnectionData::new(Some("A".into())).with_choice_key("a"),
            )
            .unwrap();
        graph
            .connect(
                choice,
                branch_b,
                ConnectionData::new(Some("B".into())).with_choice_key("b"),
            )
            .unwrap();
        graph.set_start_node(start).unwrap();

        (graph, start, choice, branch_a, branch_b)
    }

    #[test]
    fn add_node_assigns_stable_identifier() {
        let mut graph = DialogueGraph::new();
        let first = graph.add_node(DialogueNode::text("First"));
        let second = graph.add_node(DialogueNode::text("Second"));

        assert_ne!(first, second);
        assert!(graph.contains_node(first));
        assert!(graph.contains_node(second));

        assert_eq!(
            graph.get_node(first).and_then(|node| match node {
                DialogueNode::Text { text, .. } => Some(text),
                _ => None,
            }),
            Some(&"First".to_string())
        );
    }

    #[test]
    fn validate_requires_reachability() {
        let (mut graph, start, _, branch_a, _) = build_sample_graph();
        assert!(graph.validate().is_ok());

        let isolated = graph.add_node(DialogueNode::text("Isolated"));
        assert!(graph.validate().is_err());

        graph
            .connect(branch_a, isolated, ConnectionData::new(None))
            .unwrap();
        assert!(graph.validate().is_ok());

        graph.remove_node(start).unwrap();
        assert!(graph.validate().is_err());
    }

    #[test]
    fn connect_and_disconnect_edges() {
        let (mut graph, start, _, branch_a, branch_b) = build_sample_graph();

        assert_eq!(graph.get_connected_nodes(start).len(), 1);
        assert!(graph.disconnect(start, branch_a).is_err());

        graph
            .connect(start, branch_a, ConnectionData::new(Some("direct".into())))
            .unwrap();
        assert_eq!(graph.get_connected_nodes(start).len(), 2);

        graph.disconnect(start, branch_a).unwrap();
        assert_eq!(graph.get_connected_nodes(start).len(), 1);

        assert!(graph.disconnect(start, branch_a).is_err());
        assert!(
            graph
                .connect(branch_b, NodeId::from_raw(999), ConnectionData::new(None))
                .is_err()
        );
    }

    #[test]
    fn removing_start_node_clears_assignment() {
        let (mut graph, start, _, _, _) = build_sample_graph();
        assert_eq!(graph.start_node, Some(start));
        graph.remove_node(start).unwrap();
        assert!(graph.start_node.is_none());
    }

    #[test]
    fn connection_ordering_is_stable_and_reorderable() {
        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::text("Start"));
        let branch_a = graph.add_node(DialogueNode::text("A"));
        let branch_b = graph.add_node(DialogueNode::text("B"));
        let branch_c = graph.add_node(DialogueNode::text("C"));

        graph
            .connect(start, branch_b, ConnectionData::new(Some("B".into())))
            .unwrap();
        graph
            .connect(start, branch_a, ConnectionData::new(Some("A".into())))
            .unwrap();
        graph
            .connect(start, branch_c, ConnectionData::new(Some("C".into())))
            .unwrap();

        let order: Vec<_> = graph
            .get_connected_nodes(start)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(order, vec![branch_b, branch_a, branch_c]);

        graph
            .reorder_connections(start, &[branch_a, branch_b, branch_c])
            .unwrap();

        let reordered: Vec<_> = graph
            .get_connected_nodes(start)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(reordered, vec![branch_a, branch_b, branch_c]);
    }

    #[test]
    fn serialization_round_trip_preserves_structure() {
        let (graph, _, choice, _, _) = build_sample_graph();
        let json = serde_json::to_string(&graph).unwrap();
        let restored: DialogueGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(graph.node_count(), restored.node_count());
        assert_eq!(graph.name, restored.name);
        assert_eq!(graph.start_node.is_some(), restored.start_node.is_some());

        let restored_choice = restored
            .get_node(choice)
            .expect("choice node should survive round trip");
        assert_eq!(
            restored_choice.presentation_key(),
            Some("inline_badges"),
            "choice presentation key should survive round trip"
        );

        let restored_connections = restored.get_outgoing_connections(choice);
        assert_eq!(restored_connections.len(), 2);
        assert_eq!(
            restored_connections[0].1.choice_key.as_deref(),
            Some("a"),
            "choice connection key should survive round trip"
        );
    }

    #[test]
    fn validation_rejects_duplicate_choice_keys_on_single_choice_node() {
        let (mut graph, _, choice, _, branch_b) = build_sample_graph();
        let branch_c = graph.add_node(DialogueNode::text("C"));
        graph
            .connect(
                choice,
                branch_c,
                ConnectionData::new(Some("C".into())).with_choice_key("b"),
            )
            .expect("connect duplicate choice key");
        graph
            .connect(branch_b, branch_c, ConnectionData::new(None))
            .expect("keep branch reachable");

        let error = graph
            .validate()
            .expect_err("duplicate keys should fail validation");
        assert!(
            error.contains("duplicate choice_key"),
            "unexpected error message: {error}"
        );
    }
}
