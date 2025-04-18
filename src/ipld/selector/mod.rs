// Copyright 2019-2025 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

mod empty_map;
use std::ops::SubAssign;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use super::Ipld;
#[cfg(test)]
use Selector::*;

/// Selectors are expressions that identify and select a subset of data from an
/// IPLD DAG. Selectors are themselves IPLD and can be serialized and
/// de-serialized as such.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Selector {
    /// `Matcher` marks a node to be included in the "result" set.
    /// (All nodes traversed by a selector are in the "covered" set (which is
    /// a.k.a. "the Merkle proof"); the "result" set is a subset of the
    /// "covered" set.)
    ///
    /// In libraries using selectors, the "result" set is typically provided to
    /// some user-specified callback.
    ///
    /// A selector tree with only "explore*"-type selectors and no `Matcher`
    /// selectors is valid; it will just generate a "covered" set of nodes
    /// and no "result" set.
    #[serde(rename = ".", with = "empty_map")]
    Matcher,

    /// `ExploreAll` is similar to a `*` -- it traverses all elements of an
    /// array, or all entries in a map, and applies a next selector to the
    /// reached nodes.
    #[serde(rename = "a")]
    ExploreAll {
        #[serde(rename = ">")]
        next: Box<Selector>,
    },

    /// `ExploreFields` traverses named fields in a map (or equivalently,
    /// structure, if traversing on typed/schema nodes) and applies a next
    /// selector to the reached nodes.
    ///
    /// Note that a concept of exploring a whole path (e.g. "path/to/file") can
    /// be represented as a set of three nested `ExploreFields` selectors,
    /// each specifying one field.
    ///
    /// Fields insertion order is maintained and traversed using that order.
    #[serde(rename = "f")]
    ExploreFields {
        #[serde(rename = "f>")]
        fields: IndexMap<String, Selector>,
    },

    /// `ExploreIndex` traverses a specific index in a list, and applies a next
    /// selector to the reached node.
    #[serde(rename = "i")]
    ExploreIndex {
        #[serde(rename = "i")]
        index: usize,
        #[serde(rename = ">")]
        next: Box<Selector>,
    },

    /// `ExploreRange` traverses a list, and for each element in the range
    /// specified, will apply a next selector to those reached nodes.
    #[serde(rename = "r")]
    ExploreRange {
        #[serde(rename = "^")]
        start: usize,
        #[serde(rename = "$")]
        end: usize,
        #[serde(rename = ">")]
        next: Box<Selector>,
    },

    /// `ExploreRecursive` traverses some structure recursively.
    /// To guide this exploration, it uses a "sequence", which is another
    /// Selector tree; some leaf node in this sequence should contain an
    /// `ExploreRecursiveEdge` selector, which denotes the place recursion
    /// should occur.
    ///
    /// In implementation, whenever evaluation reaches an `ExploreRecursiveEdge`
    /// marker in the recursion sequence's Selector tree, the implementation
    /// logically produces another new Selector which is a copy of the
    /// original `ExploreRecursive` selector, but with a decremented depth
    /// parameter for limit (if limit is of type depth), and continues
    /// evaluation.
    ///
    /// It is not valid for an `ExploreRecursive` selector's sequence to contain
    /// no instances of `ExploreRecursiveEdge`; it *is* valid for it to contain
    /// more than one `ExploreRecursiveEdge`.
    ///
    /// `ExploreRecursive` can contain a nested `ExploreRecursive`!
    /// This is comparable to a nested for-loop.
    /// In these cases, any `ExploreRecursiveEdge` instance always refers to the
    /// nearest parent `ExploreRecursive` (in other words,
    /// `ExploreRecursiveEdge` can be thought of like the 'continue'
    /// statement, or end of a for-loop body; it is *not* a `goto`
    /// statement).
    ///
    /// Be careful when using `ExploreRecursive` with a large depth limit
    /// parameter; it can easily cause very large traversals (especially if
    /// used in combination with selectors like `ExploreAll` inside the
    /// sequence).
    ///
    /// limit is a union type -- it can have an integer depth value (key
    /// "depth") or no value (key "none"). If limit has no value it is up to
    /// the implementation library using selectors to identify an
    /// appropriate max depth as necessary so that recursion is not infinite
    #[serde(rename = "R")]
    ExploreRecursive {
        #[serde(rename = ":>")]
        sequence: Box<Selector>,
        #[serde(rename = "l")]
        limit: RecursionLimit,
        /// if a node matches, we won't match it nor explore its children.
        #[serde(rename = "!")]
        stop_at: Option<Condition>,
        #[serde(skip_deserializing)]
        /// Used to index current
        current: Option<Box<Selector>>,
    },

    /// `ExploreUnion` allows selection to continue with two or more distinct
    /// selectors while exploring the same tree of data.
    ///
    /// `ExploreUnion` can be used to apply a `Matcher` on one node (causing it
    /// to be considered part of a (possibly labeled) result set), while
    /// simultaneously continuing to explore deeper parts of the tree with
    /// another selector, for example.
    #[serde(rename = "|")]
    ExploreUnion(Vec<Selector>),

    /// `ExploreRecursiveEdge` is a special sentinel value which is used to mark
    /// the end of a sequence started by an `ExploreRecursive` selector: the
    /// recursion goes back to the initial state of the earlier
    /// `ExploreRecursive` selector, and proceeds again (with a decremented
    /// `maxDepth` value).
    ///
    /// An `ExploreRecursive` selector that doesn't contain an
    /// `ExploreRecursiveEdge` is nonsensical.  Containing more than one
    /// `ExploreRecursiveEdge` is valid. An `ExploreRecursiveEdge` without
    /// an enclosing `ExploreRecursive` is an error.
    #[serde(rename = "@", with = "empty_map")]
    ExploreRecursiveEdge,
    //* No conditional explore impl exists, ignore for now
    // #[serde(rename = "&")]
    // ExploreConditional {
    //     #[serde(rename = "&")]
    //     condition: Option<Condition>,
    //     #[serde(rename = ">")]
    //     next: Box<Selector>,
    // },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Copy)]
pub enum RecursionLimit {
    #[serde(rename = "none", with = "empty_map")]
    None,
    #[serde(rename = "depth")]
    Depth(u64),
}

impl SubAssign<u64> for RecursionLimit {
    fn sub_assign(&mut self, other: u64) {
        if let RecursionLimit::Depth(v) = self {
            *v -= other;
        }
    }
}

/// Condition is expresses a predicate with a boolean result.
///
/// Condition clauses are used several places:
///   - in `Matcher`, to determine if a node is selected.
///   - in `ExploreRecursive`, to halt exploration.
///   - in `ExploreConditional`,
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Copy)]
pub enum Condition {
    #[serde(rename = "hasField")]
    HasField,
    #[serde(rename = "=")]
    HasValue,
    #[serde(rename = "%")]
    HasKind,
    #[serde(rename = "/")]
    IsLink,
    #[serde(rename = "greaterThan")]
    GreaterThan,
    #[serde(rename = "lessThan")]
    LessThan,
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[cfg(test)]
impl Selector {
    /// Processes and returns resultant selector node
    pub fn explore(self, ipld: &Ipld, p: &str) -> Option<Selector> {
        match self {
            ExploreAll { next } => Some(*next),
            ExploreFields { mut fields } => match ipld {
                Ipld::Map(m) => {
                        m.get(p)?;
                        fields.swap_remove(p)
                    }
                ,
                // Using ExploreFields for list is supported feature in go impl
                Ipld::List(l) => {
                    // Check to make sure index is within bounds
                    if p.parse::<usize>().ok()? >= l.len() {
                        return None;
                    }
                    fields.swap_remove(p)
                }
                _ => None,
            },
            ExploreIndex { index, next } => match ipld {
                Ipld::List(l) => {
                    let i = p.parse::<usize>().ok()?;
                    if i != index || i >= l.len() {
                        None
                    } else {
                        // Path segment matches selector index
                        Some(*next)
                    }
                }
                _ => None,
            },
            ExploreRange { start, end, next } => {
                match ipld {
                    Ipld::List(l) => {
                        let i = p.parse::<usize>().ok()?;
                        // Check to make sure index is within list bounds
                        if i < start || i >= end || i >= l.len() {
                            None
                        } else {
                            // Path segment is within the selector range
                            Some(*next)
                        }
                    }
                    _ => None,
                }
            }
            ExploreRecursive {
                current,
                sequence,
                mut limit,
                stop_at,
            } => {
                let next = current
                    .unwrap_or_else(|| sequence.clone())
                    .explore(ipld, p)?;

                if !has_recursive_edge(&next) {
                    return Some(ExploreRecursive {
                        sequence,
                        current: Some(next.into()),
                        limit,
                        stop_at,
                    });
                }

                if let RecursionLimit::Depth(depth) = limit {
                    if depth < 2 {
                        // Replaces recursive edge with None on last iteration
                        return replace_recursive_edge(next, None);
                    }
                    limit -= 1;
                }

                Some(ExploreRecursive {
                    current: replace_recursive_edge(next, Some(*sequence.clone())).map(Box::new),
                    sequence,
                    limit,
                    stop_at,
                })
            }
            ExploreUnion(selectors) => {
                // Push all valid explored selectors to new vector
                let replace_selectors: Vec<_> = selectors
                    .into_iter()
                    .filter_map(|s| s.explore(ipld, p))
                    .collect();

                Selector::from_selectors(replace_selectors)
            }
            // Go impl panics here, but panic on exploring malformed selector seems bad
            ExploreRecursiveEdge => None,
            // Matcher is terminal selector
            Matcher => None,
        }
    }

    fn from_selectors(mut vec: Vec<Self>) -> Option<Self> {
        match vec.len() {
            0 | 1 => vec.pop(),
            _ => Some(ExploreUnion(vec)),
        }
    }
}

#[cfg(test)]
fn replace_recursive_edge(next_sel: Selector, replace: Option<Selector>) -> Option<Selector> {
    match next_sel {
        ExploreRecursiveEdge => replace,
        ExploreUnion(selectors) => {
            // Push all valid explored selectors to new vector
            let replace_selectors: Vec<_> = selectors
                .into_iter()
                .filter_map(|s| replace_recursive_edge(s, replace.clone()))
                .collect();

            Selector::from_selectors(replace_selectors)
        }
        _ => Some(next_sel),
    }
}
#[cfg(test)]
fn has_recursive_edge(next_sel: &Selector) -> bool {
    match next_sel {
        ExploreRecursiveEdge { .. } => true,
        ExploreUnion(selectors) => selectors.iter().any(has_recursive_edge),
        _ => false,
    }
}
