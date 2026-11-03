//! Lane layout for the commit graph.
//!
//! Commits arrive newest first, every child above its parents. Each lane
//! remembers the commit it is heading towards. A commit takes the leftmost lane
//! waiting for it, and every other lane waiting for it merges in. Below the
//! commit, its first parent continues in the same lane; any other parent joins a
//! lane already heading there, or opens the leftmost free one.

use gix::ObjectId;

/// A piece of line inside one row of the graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    /// A lane passing the row untouched, from the top edge to the bottom edge.
    Pass(u16),
    /// A line from lane `from` at the top edge into the row's commit.
    In { from: u16 },
    /// A line from the row's commit to lane `to` at the bottom edge.
    Out { to: u16 },
}

impl Edge {
    fn lane(self) -> u16 {
        match self {
            Edge::Pass(lane) | Edge::In { from: lane } | Edge::Out { to: lane } => lane,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Row {
    /// The lane holding this row's commit.
    pub(crate) lane: u16,
    pub(crate) merge: bool,
    pub(crate) edges: Vec<Edge>,
    /// The number of lanes this row draws in.
    pub(crate) width: u16,
}

#[derive(Debug, Default)]
pub(crate) struct Graph {
    pub(crate) rows: Vec<Row>,
    /// The number of lanes the widest row uses.
    pub(crate) width: usize,
}

pub(crate) fn layout<'a>(
    commits: impl IntoIterator<Item = (&'a ObjectId, &'a [ObjectId])>,
) -> Graph {
    let mut lanes: Vec<Option<ObjectId>> = Vec::new();
    let mut graph = Graph::default();
    for (id, parents) in commits {
        let lane = match lanes.iter().position(|awaited| awaited.as_ref() == Some(id)) {
            Some(lane) => lane,
            None => free_lane(&mut lanes),
        };

        let mut edges = Vec::new();
        for (ix, awaited) in lanes.iter_mut().enumerate() {
            if *awaited == Some(*id) {
                edges.push(Edge::In { from: ix as u16 });
                *awaited = None;
            } else if awaited.is_some() {
                edges.push(Edge::Pass(ix as u16));
            }
        }

        if let Some((first, others)) = parents.split_first() {
            lanes[lane] = Some(*first);
            edges.push(Edge::Out { to: lane as u16 });
            for parent in others {
                let to = match lanes.iter().position(|awaited| awaited.as_ref() == Some(parent)) {
                    Some(to) => to,
                    None => {
                        let to = free_lane(&mut lanes);
                        lanes[to] = Some(*parent);
                        to
                    }
                };
                edges.push(Edge::Out { to: to as u16 });
            }
        }

        let used = edges
            .iter()
            .map(|edge| edge.lane() as usize + 1)
            .max()
            .unwrap_or(0)
            .max(lane + 1);
        graph.width = graph.width.max(used);
        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }
        graph.rows.push(Row {
            lane: lane as u16,
            merge: parents.len() > 1,
            edges,
            width: used as u16,
        });
    }
    graph
}

/// The leftmost free lane, opening one on the right when every lane is taken.
fn free_lane(lanes: &mut Vec<Option<ObjectId>>) -> usize {
    match lanes.iter().position(Option::is_none) {
        Some(lane) => lane,
        None => {
            lanes.push(None);
            lanes.len() - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(byte: u8) -> ObjectId {
        ObjectId::from_bytes_or_panic(&[byte; 20])
    }

    fn graph(commits: &[(ObjectId, Vec<ObjectId>)]) -> Graph {
        layout(commits.iter().map(|(id, parents)| (id, parents.as_slice())))
    }

    #[test]
    fn linear_history_stays_in_one_lane() {
        let (a, b, c) = (id(1), id(2), id(3));
        let graph = graph(&[(a, vec![b]), (b, vec![c]), (c, vec![])]);
        assert_eq!(graph.width, 1);
        assert!(graph.rows.iter().all(|row| row.lane == 0 && !row.merge));
        assert_eq!(graph.rows[0].edges, [Edge::Out { to: 0 }]);
        assert_eq!(graph.rows[1].edges, [Edge::In { from: 0 }, Edge::Out { to: 0 }]);
        assert_eq!(graph.rows[2].edges, [Edge::In { from: 0 }]);
    }

    #[test]
    fn merge_opens_a_lane_that_closes_at_the_fork_point() {
        // `merge` joins `feature` into the line that continues to `base`.
        let (merge, feature, base) = (id(1), id(2), id(3));
        let graph = graph(&[
            (merge, vec![base, feature]),
            (feature, vec![base]),
            (base, vec![]),
        ]);
        assert_eq!(graph.width, 2);
        assert!(graph.rows[0].merge);
        assert_eq!(graph.rows[0].edges, [Edge::Out { to: 0 }, Edge::Out { to: 1 }]);
        assert_eq!(graph.rows[1].lane, 1);
        assert_eq!(
            graph.rows[1].edges,
            [Edge::Pass(0), Edge::In { from: 1 }, Edge::Out { to: 1 }]
        );
        assert_eq!(graph.rows[2].lane, 0);
        assert_eq!(graph.rows[2].edges, [Edge::In { from: 0 }, Edge::In { from: 1 }]);
    }

    #[test]
    fn a_second_tip_opens_beside_the_lane_it_passes() {
        let (first, second, base) = (id(1), id(2), id(3));
        let graph = graph(&[(first, vec![base]), (second, vec![base]), (base, vec![])]);
        assert_eq!(graph.rows[1].lane, 1);
        assert_eq!(graph.rows[1].edges, [Edge::Pass(0), Edge::Out { to: 1 }]);
        assert_eq!(graph.rows[2].edges, [Edge::In { from: 0 }, Edge::In { from: 1 }]);
    }

    #[test]
    fn a_freed_lane_is_reused() {
        // After `side` reaches its root, the next tip takes its lane back.
        let (main, side, root, tip) = (id(1), id(2), id(3), id(4));
        let graph = graph(&[
            (main, vec![root]),
            (side, vec![]),
            (tip, vec![root]),
            (root, vec![]),
        ]);
        assert_eq!(graph.rows[1].lane, 1);
        assert_eq!(graph.rows[2].lane, 1);
        assert_eq!(graph.width, 2);
    }
}
