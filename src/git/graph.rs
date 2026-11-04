use gix::ObjectId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Pass(u16),
    In { from: u16 },
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
    pub(crate) node_lane: u16,
    pub(crate) is_merge: bool,
    pub(crate) edges: Vec<Edge>,
    pub(crate) lanes: u16,
}

#[derive(Debug, Default)]
pub(crate) struct Graph {
    pub(crate) rows: Vec<Row>,
    pub(crate) max_lanes: usize,
}

pub(crate) fn layout<'a>(
    commits: impl IntoIterator<Item = (&'a ObjectId, &'a [ObjectId])>,
) -> Graph {
    let mut awaited_by_lane: Vec<Option<ObjectId>> = Vec::new();
    let mut graph = Graph::default();
    for (id, parents) in commits {
        let node_lane = lane_awaiting(&awaited_by_lane, id)
            .unwrap_or_else(|| leftmost_free_lane(&mut awaited_by_lane));

        let mut edges = Vec::new();
        for (lane, awaited) in awaited_by_lane.iter_mut().enumerate() {
            if *awaited == Some(*id) {
                edges.push(Edge::In { from: lane as u16 });
                *awaited = None;
            } else if awaited.is_some() {
                edges.push(Edge::Pass(lane as u16));
            }
        }

        if let Some((first, others)) = parents.split_first() {
            awaited_by_lane[node_lane] = Some(*first);
            edges.push(Edge::Out {
                to: node_lane as u16,
            });
            for parent in others {
                let to = lane_awaiting(&awaited_by_lane, parent).unwrap_or_else(|| {
                    let lane = leftmost_free_lane(&mut awaited_by_lane);
                    awaited_by_lane[lane] = Some(*parent);
                    lane
                });
                edges.push(Edge::Out { to: to as u16 });
            }
        }

        let lanes = edges
            .iter()
            .map(|edge| edge.lane() as usize + 1)
            .max()
            .unwrap_or(0)
            .max(node_lane + 1);
        graph.max_lanes = graph.max_lanes.max(lanes);
        while awaited_by_lane.last().is_some_and(Option::is_none) {
            awaited_by_lane.pop();
        }
        graph.rows.push(Row {
            node_lane: node_lane as u16,
            is_merge: parents.len() > 1,
            edges,
            lanes: lanes as u16,
        });
    }
    graph
}

fn lane_awaiting(awaited_by_lane: &[Option<ObjectId>], id: &ObjectId) -> Option<usize> {
    awaited_by_lane
        .iter()
        .position(|awaited| awaited.as_ref() == Some(id))
}

fn leftmost_free_lane(awaited_by_lane: &mut Vec<Option<ObjectId>>) -> usize {
    match awaited_by_lane.iter().position(Option::is_none) {
        Some(lane) => lane,
        None => {
            awaited_by_lane.push(None);
            awaited_by_lane.len() - 1
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
        assert_eq!(graph.max_lanes, 1);
        assert!(
            graph
                .rows
                .iter()
                .all(|row| row.node_lane == 0 && !row.is_merge)
        );
        assert_eq!(graph.rows[0].edges, [Edge::Out { to: 0 }]);
        assert_eq!(
            graph.rows[1].edges,
            [Edge::In { from: 0 }, Edge::Out { to: 0 }]
        );
        assert_eq!(graph.rows[2].edges, [Edge::In { from: 0 }]);
    }

    #[test]
    fn merge_opens_a_lane_that_closes_at_the_fork_point() {
        let (merge, feature, base) = (id(1), id(2), id(3));
        let graph = graph(&[
            (merge, vec![base, feature]),
            (feature, vec![base]),
            (base, vec![]),
        ]);
        assert_eq!(graph.max_lanes, 2);
        assert!(graph.rows[0].is_merge);
        assert_eq!(
            graph.rows[0].edges,
            [Edge::Out { to: 0 }, Edge::Out { to: 1 }]
        );
        assert_eq!(graph.rows[1].node_lane, 1);
        assert_eq!(
            graph.rows[1].edges,
            [Edge::Pass(0), Edge::In { from: 1 }, Edge::Out { to: 1 }]
        );
        assert_eq!(graph.rows[2].node_lane, 0);
        assert_eq!(
            graph.rows[2].edges,
            [Edge::In { from: 0 }, Edge::In { from: 1 }]
        );
    }

    #[test]
    fn a_second_tip_opens_beside_the_lane_it_passes() {
        let (first, second, base) = (id(1), id(2), id(3));
        let graph = graph(&[(first, vec![base]), (second, vec![base]), (base, vec![])]);
        assert_eq!(graph.rows[1].node_lane, 1);
        assert_eq!(graph.rows[1].edges, [Edge::Pass(0), Edge::Out { to: 1 }]);
        assert_eq!(
            graph.rows[2].edges,
            [Edge::In { from: 0 }, Edge::In { from: 1 }]
        );
    }

    #[test]
    fn a_lane_freed_at_a_root_is_taken_by_the_next_tip() {
        let (main, side, root, tip) = (id(1), id(2), id(3), id(4));
        let graph = graph(&[
            (main, vec![root]),
            (side, vec![]),
            (tip, vec![root]),
            (root, vec![]),
        ]);
        assert_eq!(graph.rows[1].node_lane, 1);
        assert_eq!(graph.rows[2].node_lane, 1);
        assert_eq!(graph.max_lanes, 2);
    }
}
