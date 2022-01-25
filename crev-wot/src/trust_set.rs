use std::cmp;

use super::*;
use itertools::Itertools;

/// Details of a one Id that is trusted
#[derive(Debug, Clone)]
pub struct TrustedIdDetails {
    // distanc from the root of trust
    pub distance: u64,
    // effective, global trust from the root of the WoT
    pub effective_trust_level: TrustLevel,
    /// People that reported trust for this id
    pub reported_by: HashMap<Id, TrustLevel>,
}

/// Details of a one Id that is distrusted
#[derive(Debug, Clone, Default)]
pub struct DistrustedIdDetails {
    /// People that reported distrust for this id
    pub reported_by: HashSet<Id>,
}

#[derive(Debug, Clone)]
pub enum TraverseLogItem {
    Node(TraverseLogNode),
    Edge(TraverseLogEdge),
}

impl From<TraverseLogEdge> for TraverseLogItem {
    fn from(e: TraverseLogEdge) -> Self {
        TraverseLogItem::Edge(e)
    }
}
impl From<TraverseLogNode> for TraverseLogItem {
    fn from(e: TraverseLogNode) -> Self {
        TraverseLogItem::Node(e)
    }
}

#[derive(Debug, Clone)]
pub struct TraverseLogEdge {
    pub from: Id,
    pub to: Id,
    pub direct_trust: TrustLevel,
    pub effective_trust: TrustLevel,
    pub relative_distance: Option<u64>,
    pub total_distance: Option<u64>,
    pub distrusted_by: HashSet<Id>,
    pub overriden_by: HashSet<Id>,

    pub no_change: bool,
    pub ignored_distrusted: bool,
    pub ignored_trust_too_low: bool,
    pub ignored_overriden: bool,
    pub ignored_too_far: bool,
}

#[derive(Debug, Clone)]
pub struct TraverseLogNode {
    pub id: Id,
    pub effective_trust: TrustLevel,
    pub total_distance: u64,
}

#[derive(Default, Debug, Clone)]
pub struct TrustSet {
    pub traverse_log: Vec<TraverseLogItem>,

    pub trusted: HashMap<Id, TrustedIdDetails>,
    pub distrusted: HashMap<Id, DistrustedIdDetails>,

    // "ignore trust from `Id` to `Id`, as overriden by some other Ids with an effective `TrustLevel`s
    pub trust_ignore_overrides: HashMap<(Id, Id), OverrideSourcesDetails>,

    // "ignore specific package review by `Id`, as overriden by some other Ids with an effective `TrustLevel`s
    pub package_review_ignore_override: HashMap<PkgVersionReviewId, OverrideSourcesDetails>,
}

impl TrustSet {
    pub fn from(db: &ProofDB, for_id: &Id, params: &TrustDistanceParams) -> TrustSet {
        let mut distrusted = HashMap::new();

        // We keep retrying the whole thing, with more and more
        // distrusted Ids
        loop {
            let prev_distrusted_len = distrusted.len();
            let trust_set = Self::from_inner_loop(db, for_id, params, distrusted);
            if trust_set.distrusted.len() <= prev_distrusted_len {
                return trust_set;
            }
            distrusted = trust_set.distrusted;
        }
    }

    fn log_traverse(&mut self, item: impl Into<TraverseLogItem>) {
        self.traverse_log.push(item.into());
    }

    /// Calculate the effective trust levels for IDs inside a WoT.
    ///
    /// This is one of the most important functions in `crev-wot`.
    fn from_inner_loop(
        db: &ProofDB,
        for_id: &Id,
        params: &TrustDistanceParams,
        distrusted: HashMap<Id, DistrustedIdDetails>,
    ) -> Self {
        /// Node that is to be visited
        ///
        /// Order of field is important, since we use the `Ord` trait
        /// to visit nodes breadth-first with respect to trust level
        #[derive(Eq, PartialEq, Clone, Debug)]
        struct Visit {
            /// Effective transitive trust level of the node
            effective_trust_level: TrustLevel,
            /// Distance from the root, in some abstract numerical unit
            distance: u64,
            /// Id we're visit
            id: Id,
        }

        impl cmp::Ord for Visit {
            fn cmp(&self, other: &Self) -> cmp::Ordering {
                self.effective_trust_level
                    .cmp(&other.effective_trust_level)
                    .reverse()
                    .then(self.distance.cmp(&other.distance))
                    .then_with(|| self.id.cmp(&other.id))
            }
        }

        impl cmp::PartialOrd for Visit {
            fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut pending = BTreeSet::new();
        let mut current_trust_set = TrustSet::default();
        let initial_distrusted_len = distrusted.len();
        current_trust_set.distrusted = distrusted;

        pending.insert(Visit {
            effective_trust_level: TrustLevel::High,
            distance: 0,
            id: for_id.clone(),
        });
        let mut previous_iter_trust_level = TrustLevel::High;
        current_trust_set.record_trusted_id(for_id.clone(), for_id.clone(), 0, TrustLevel::High);

        while let Some(current) = pending.iter().next().cloned() {
            debug!("Traversing id: {:?}", current);
            pending.remove(&current);
            current_trust_set.log_traverse(TraverseLogNode {
                id: current.id.clone(),
                effective_trust: current.effective_trust_level,
                total_distance: current.distance,
            });

            // did we just move from processing higher effective trust level id to a lower ones?
            // if so, now would be a good time to check if anyone got banned, and then restart processing
            // of the WoT
            if current.effective_trust_level != previous_iter_trust_level {
                debug!(
                    "No more nodes with effective_trust_level of {}",
                    previous_iter_trust_level
                );
                assert!(current.effective_trust_level < previous_iter_trust_level);
                if initial_distrusted_len != current_trust_set.distrusted.len() {
                    debug!("Some people got banned at the current trust level - restarting the WoT calculation");
                    break;
                }
            }
            previous_iter_trust_level = current.effective_trust_level;

            for pkg_review in db.get_package_reviews_by_author(&current.id) {
                for override_ in &pkg_review.override_ {
                    current_trust_set
                        .package_review_ignore_override
                        .entry(PkgVersionReviewId {
                            from: override_.id.id.clone(),
                            package_version_id: pkg_review.package.id.clone(),
                        })
                        .or_default()
                        .insert(current.id.clone(), current.effective_trust_level);
                }
            }

            for (trust_details, candidate_id) in db.get_trust_details_list_of_id(&current.id) {
                let direct_trust = trust_details.level;
                let current_overrides = &trust_details.override_;

                // Note: we keep visiting nodes, even banned ones, just like they were originally
                // reported
                let effective_trust_level =
                    std::cmp::min(direct_trust, current.effective_trust_level);
                debug!(
                    "Effective trust for {} {}",
                    candidate_id, effective_trust_level
                );

                let candidate_distance_from_current =
                    params.distance_by_level(effective_trust_level);

                let candidate_total_distance = candidate_distance_from_current
                    .map(|rel_distance| rel_distance + current.distance);

                let distrusted_by = current_trust_set
                    .distrusted
                    .get(candidate_id)
                    .map(ToOwned::to_owned)
                    .map(|details| details.reported_by)
                    .unwrap_or_else(HashSet::new);

                let too_far = candidate_total_distance.map(|d| params.max_distance < d);
                let trust_too_low = effective_trust_level == TrustLevel::None;

                let overriden_by = if let Some(existing_override) = current_trust_set
                    .trust_ignore_overrides
                    .get(&(current.id.clone(), candidate_id.clone()))
                {
                    if current.effective_trust_level
                        < existing_override.max_level().expect("must not be empty")
                    {
                        existing_override.0.keys().cloned().collect()
                    } else {
                        HashSet::new()
                    }
                } else {
                    HashSet::new()
                };

                current_trust_set.log_traverse(TraverseLogEdge {
                    from: current.id.clone(),
                    to: candidate_id.clone(),
                    direct_trust,
                    effective_trust: effective_trust_level,
                    relative_distance: candidate_distance_from_current,
                    total_distance: candidate_total_distance,
                    distrusted_by: distrusted_by.clone(),
                    overriden_by: overriden_by.clone(),

                    ignored_distrusted: !distrusted_by.is_empty(),
                    ignored_too_far: too_far.unwrap_or(true),
                    ignored_trust_too_low: trust_too_low,
                    ignored_overriden: !overriden_by.is_empty(),

                    // to be changed if there was actually a change
                    no_change: true,
                });

                debug!(
                    "{} ({}) reports trust level for {}: {}",
                    current.id, current.effective_trust_level, candidate_id, direct_trust
                );

                if !distrusted_by.is_empty() {
                    debug!(
                        "{} is distrusted by {} (reported_by: {})",
                        candidate_id,
                        current.id,
                        distrusted_by.iter().map(|id| id.to_string()).join(", ")
                    );
                    continue;
                }

                if !overriden_by.is_empty() {
                    debug!(
                        "{} trust for {} was ignored (overriden)",
                        current.id, candidate_id
                    );
                    continue;
                }
                // Note: lower trust node can ban higher trust node, but only
                // if it wasn't banned by a higher trust node beforehand.
                // However banning by the same trust level node, does not prevent
                // the node from banning others.
                if direct_trust == TrustLevel::Distrust {
                    debug!(
                        "Adding {} to distrusted list (via {})",
                        candidate_id, current.id
                    );
                    // We discard the result, because we actually want to make as much
                    // progress as possible before restaring building the WoT, and
                    // we will not visit any node that was marked as distrusted,
                    // becuse we check it for every node to be visited
                    let _ = current_trust_set
                        .record_distrusted_id(candidate_id.clone(), current.id.clone());

                    continue;
                }

                for override_item in current_overrides {
                    let trust_ignore_override = (override_item.clone(), candidate_id.clone());
                    current_trust_set
                        .trust_ignore_overrides
                        .entry(trust_ignore_override)
                        .or_default()
                        .insert(current.id.clone(), current.effective_trust_level);
                }

                if trust_too_low {
                    continue;
                } else if effective_trust_level < TrustLevel::None {
                    unreachable!(
                        "this should not happen: candidate_effective_trust <= TrustLevel::None"
                    );
                }

                let candidate_distance_from_current =
                    if let Some(v) = candidate_distance_from_current {
                        v
                    } else {
                        debug!("Not traversing {}: trust too low", candidate_id);
                        continue;
                    };
                let candidate_total_distance =
                    candidate_total_distance.expect("should not be empty");

                debug!(
                    "Distance of {} from {}: {}. Total distance from root: {}.",
                    candidate_id,
                    current.id,
                    candidate_distance_from_current,
                    candidate_total_distance
                );

                if too_far.expect("should not be empty") {
                    debug!(
                        "Total distance of {}: {} higher than max_distance: {}.",
                        candidate_id, candidate_total_distance, params.max_distance
                    );
                    continue;
                }

                let prev_trust_details = current_trust_set.trusted.get(candidate_id).cloned();

                if current_trust_set.record_trusted_id(
                    candidate_id.clone(),
                    current.id.clone(),
                    candidate_total_distance,
                    effective_trust_level,
                ) {
                    if let Some(TraverseLogItem::Edge(edge)) =
                        current_trust_set.traverse_log.last_mut()
                    {
                        edge.no_change = false;
                    } else {
                        unreachable!("Wrong type of last TraverseLogItem");
                    }

                    // to avoid visiting same node multiple times, remove
                    // any existing pending `Visit` using previous trust details
                    if let Some(prev_trust_details) = prev_trust_details {
                        pending.remove(&Visit {
                            id: candidate_id.clone(),
                            distance: prev_trust_details.distance,
                            effective_trust_level: prev_trust_details.effective_trust_level,
                        });
                    }
                    let visit = Visit {
                        effective_trust_level,
                        distance: candidate_total_distance,
                        id: candidate_id.to_owned(),
                    };
                    // we've just removed it above, so can't return true
                    assert!(pending.insert(visit));
                }
            }
        }

        current_trust_set
    }

    pub fn iter_trusted_ids(&self) -> impl Iterator<Item = &Id> {
        self.trusted.keys()
    }

    pub fn get_trusted_ids(&self) -> HashSet<crev_data::Id> {
        self.iter_trusted_ids().cloned().collect()
    }

    pub fn get_trusted_ids_refs(&self) -> HashSet<&crev_data::Id> {
        self.iter_trusted_ids().collect()
    }

    pub fn is_trusted(&self, id: &Id) -> bool {
        self.trusted.contains_key(id)
    }

    pub fn is_distrusted(&self, id: &Id) -> bool {
        self.distrusted.contains_key(id)
    }

    /// Record that an Id is reported as distrusted
    ///
    /// Return `true` if it was previously considered as trusted,
    /// and so that WoT traversal needs to be restarted
    fn record_distrusted_id(&mut self, subject: Id, reported_by: Id) -> bool {
        let res = self.trusted.remove(&subject).is_some();

        self.distrusted
            .entry(subject)
            .or_default()
            .reported_by
            .insert(reported_by);

        res
    }

    /// Record that an Id is reported as trusted
    ///
    /// Returns `true` if this this ID details changed in a way,
    /// which requires revising it's own downstream trusted Id details in the graph algorithm for it.
    fn record_trusted_id(
        &mut self,
        subject: Id,
        reported_by: Id,
        distance: u64,
        effective_trust_level: TrustLevel,
    ) -> bool {
        use std::collections::hash_map::Entry;

        assert!(effective_trust_level >= TrustLevel::None);

        match self.trusted.entry(subject) {
            Entry::Vacant(entry) => {
                let reported_by = vec![(reported_by, effective_trust_level)]
                    .into_iter()
                    .collect();
                entry.insert(TrustedIdDetails {
                    distance,
                    effective_trust_level,
                    reported_by,
                });
                true
            }
            Entry::Occupied(mut prev) => {
                let mut needs_revisit = false;
                let prev = prev.get_mut();
                if prev.distance > distance {
                    prev.distance = distance;
                    needs_revisit = true;
                }
                if prev.effective_trust_level < effective_trust_level {
                    prev.effective_trust_level = effective_trust_level;
                    needs_revisit = true;
                }
                match prev.reported_by.entry(reported_by) {
                    Entry::Vacant(entry) => {
                        entry.insert(effective_trust_level);
                    }
                    Entry::Occupied(mut entry) => {
                        let level = entry.get_mut();
                        if *level < effective_trust_level {
                            *level = effective_trust_level;
                        }
                    }
                }
                needs_revisit
            }
        }
    }

    pub fn get_effective_trust_level(&self, id: &Id) -> TrustLevel {
        self.get_effective_trust_level_opt(id)
            .unwrap_or(TrustLevel::None)
    }

    pub fn get_effective_trust_level_opt(&self, id: &Id) -> Option<TrustLevel> {
        self.trusted
            .get(id)
            .map(|details| details.effective_trust_level)
            .or_else(|| self.distrusted.get(id).map(|_| TrustLevel::Distrust))
    }
}
