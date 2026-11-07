//! Run-owned bounded DAG scheduling.

use std::collections::{HashMap, HashSet, VecDeque};

use futures_util::stream::{FuturesUnordered, StreamExt};

use super::{FlowConnection, FlowDefinition, FlowExecutor, FlowNodeDefinition, NodeResult};

impl FlowExecutor {
    /// Execute the flow DAG in one run-owned scheduler.
    ///
    /// Sequence follows execution edges, fan-out runs concurrently up to the
    /// flow limit, and a node with multiple incoming execution edges uses
    /// join-all semantics. Futures are not detached Tokio tasks, so cancelling
    /// the root run drops all in-flight node futures together.
    pub(super) async fn execute_flow_graph(
        &self,
        run_id: &str,
        flow: &FlowDefinition,
    ) -> Result<(), String> {
        type NodeFuture = std::pin::Pin<
            Box<dyn std::future::Future<Output = (String, Result<NodeResult, String>)> + Send>,
        >;

        let nodes_by_id: HashMap<String, FlowNodeDefinition> = flow
            .nodes
            .iter()
            .cloned()
            .map(|node| (node.id.clone(), node))
            .collect();
        let reachable = self.find_reachable_nodes_from_trigger(flow);
        let max_concurrent = flow.metadata.execution_settings.max_concurrent_nodes.max(1) as usize;

        // A recovered run already has durably completed nodes. Seed both sets
        // from that checkpoint so recovery continues at the graph frontier
        // instead of replaying entry nodes and their side effects.
        let mut finished: HashSet<String> = self
            .active_flows
            .lock()
            .await
            .get(run_id)
            .map(|state| {
                state
                    .completed_nodes
                    .intersection(&reachable)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let mut scheduled = finished.clone();
        let node_results = self
            .active_flows
            .lock()
            .await
            .get(run_id)
            .map(|state| state.node_results.clone())
            .unwrap_or_default();
        let mut dynamic_reachable = Self::reachable_for_resolved_routes(flow, &node_results);
        let mut ready = VecDeque::from(Self::ready_nodes_for_resolved_routes(
            flow,
            &dynamic_reachable,
            &finished,
            &scheduled,
            &node_results,
        ));
        let mut in_flight: FuturesUnordered<NodeFuture> = FuturesUnordered::new();

        loop {
            if self.is_run_cancelled(run_id).await {
                return Ok(());
            }

            while in_flight.len() < max_concurrent {
                let Some(node_id) = ready.pop_front() else {
                    break;
                };
                if !scheduled.insert(node_id.clone()) {
                    continue;
                }
                let node = nodes_by_id
                    .get(&node_id)
                    .cloned()
                    .ok_or_else(|| format!("Scheduled node {} does not exist", node_id))?;
                let executor = self.clone();
                let run_id = run_id.to_string();
                in_flight.push(Box::pin(async move {
                    let result = executor.execute_single_node(&run_id, &node).await;
                    (node_id, result)
                }));
            }

            let Some((node_id, result)) = in_flight.next().await else {
                break;
            };
            result?;
            finished.insert(node_id.clone());
            let node_results = self
                .active_flows
                .lock()
                .await
                .get(run_id)
                .map(|state| state.node_results.clone())
                .unwrap_or_default();
            dynamic_reachable = Self::reachable_for_resolved_routes(flow, &node_results);
            ready.retain(|candidate| dynamic_reachable.contains(candidate));
            for successor_id in Self::ready_nodes_for_resolved_routes(
                flow,
                &dynamic_reachable,
                &finished,
                &scheduled,
                &node_results,
            ) {
                if !ready.contains(&successor_id) {
                    ready.push_back(successor_id);
                }
            }
        }

        let blocked: Vec<String> = dynamic_reachable
            .difference(&finished)
            .filter(|node_id| !finished.contains(*node_id))
            .cloned()
            .collect();
        if !blocked.is_empty() && !self.is_run_cancelled(run_id).await {
            return Err(format!(
                "Flow graph could not make progress; blocked or cyclic nodes: {}",
                blocked.join(", ")
            ));
        }

        Ok(())
    }

    fn edge_is_selected(
        connection: &FlowConnection,
        node_results: &HashMap<String, NodeResult>,
    ) -> bool {
        let FlowConnection::Control {
            from_node, route, ..
        } = connection
        else {
            return true;
        };
        let selected_route = node_results
            .get(from_node)
            .and_then(|result| result.outputs.get("selectedRoute"))
            .and_then(serde_json::Value::as_str);

        match route {
            Some(route) => selected_route == Some(route.as_str()),
            None => selected_route.is_none() || selected_route == Some("output"),
        }
    }

    /// Before a control node finishes every route is potentially reachable.
    /// Once it has selected a route, the other branches disappear from the
    /// active graph. This also gives joins correct semantics when one incoming
    /// branch was intentionally not selected.
    pub(super) fn reachable_for_resolved_routes(
        flow: &FlowDefinition,
        node_results: &HashMap<String, NodeResult>,
    ) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::from([flow.start_node_id.clone()]);
        while let Some(node_id) = queue.pop_front() {
            if !reachable.insert(node_id.clone()) {
                continue;
            }
            let source_finished = node_results.get(&node_id).is_some_and(|result| {
                matches!(
                    result.status,
                    super::NodeStatus::Completed | super::NodeStatus::Failed
                )
            });
            for connection in flow
                .connections
                .iter()
                .filter(|connection| connection.from_node() == node_id && connection.is_control())
            {
                if !source_finished || Self::edge_is_selected(connection, node_results) {
                    queue.push_back(connection.to_node().to_string());
                }
            }
        }
        reachable
    }

    pub(super) fn ready_nodes_for_resolved_routes(
        flow: &FlowDefinition,
        reachable: &HashSet<String>,
        finished: &HashSet<String>,
        scheduled: &HashSet<String>,
        node_results: &HashMap<String, NodeResult>,
    ) -> Vec<String> {
        flow.nodes
            .iter()
            .filter(|node| reachable.contains(&node.id) && !scheduled.contains(&node.id))
            .filter(|node| {
                flow.connections
                    .iter()
                    .filter(|connection| {
                        connection.to_node() == node.id
                            && connection.is_control()
                            && reachable.contains(connection.from_node())
                            && (!finished.contains(connection.from_node())
                                || Self::edge_is_selected(connection, node_results))
                    })
                    .all(|connection| finished.contains(connection.from_node()))
            })
            .map(|node| node.id.clone())
            .collect()
    }

    pub(super) fn ready_nodes_at_frontier(
        flow: &FlowDefinition,
        reachable: &HashSet<String>,
        finished: &HashSet<String>,
        scheduled: &HashSet<String>,
    ) -> Vec<String> {
        flow.nodes
            .iter()
            .filter(|node| reachable.contains(&node.id) && !scheduled.contains(&node.id))
            .filter(|node| {
                flow.connections
                    .iter()
                    .filter(|connection| connection.to_node() == node.id && connection.is_control())
                    .all(|connection| finished.contains(connection.from_node()))
            })
            .map(|node| node.id.clone())
            .collect()
    }

    pub(super) fn ready_execution_successors(
        connections: &[FlowConnection],
        completed_node_id: &str,
        finished: &HashSet<String>,
        scheduled: &HashSet<String>,
    ) -> Vec<String> {
        let successor_ids: HashSet<String> = connections
            .iter()
            .filter(|connection| {
                connection.from_node() == completed_node_id && connection.is_control()
            })
            .map(|connection| connection.to_node().to_string())
            .collect();

        successor_ids
            .into_iter()
            .filter(|successor_id| !scheduled.contains(successor_id))
            .filter(|successor_id| {
                connections
                    .iter()
                    .filter(|connection| {
                        connection.to_node() == *successor_id && connection.is_control()
                    })
                    .all(|connection| finished.contains(connection.from_node()))
            })
            .collect()
    }
}
