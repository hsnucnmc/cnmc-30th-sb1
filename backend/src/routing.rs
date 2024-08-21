use packet::TrackID;
use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type RoutingStateID = u32;
pub type Weight = f64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RoutingType {
    Derail,
    BounceBack,
    Track(TrackID),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompoundRoutingType {
    Simple(RoutingType),
    Weighted(Vec<(Weight, RoutingType)>),
}

impl CompoundRoutingType {
    fn build(self) -> BuiltCompoundRoutingType {
        match self {
            CompoundRoutingType::Simple(routing) => BuiltCompoundRoutingType::Simple(routing),
            CompoundRoutingType::Weighted(routings) => {
                let weighted = WeightedIndex::new(routings.iter().map(|(x, _)| x)).unwrap();
                let routings = routings.iter().map(|(_, x)| *x).collect();
                BuiltCompoundRoutingType::Weighted(weighted, routings)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum BuiltCompoundRoutingType {
    Simple(RoutingType),
    Weighted(WeightedIndex<f64>, Vec<RoutingType>),
}

impl Distribution<RoutingType> for BuiltCompoundRoutingType {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RoutingType {
        match self {
            BuiltCompoundRoutingType::Simple(routing) => *routing,
            BuiltCompoundRoutingType::Weighted(weighted, routings) => {
                routings[weighted.sample(rng)]
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AfterEffects {
    Nothing,
    SwitchState(RoutingStateID),
    Weighted(Vec<(Weight, Option<RoutingStateID>)>),
}

impl AfterEffects {
    fn build(self) -> BuiltAfterEffects {
        match self {
            AfterEffects::Nothing => BuiltAfterEffects::Nothing,
            AfterEffects::SwitchState(state) => BuiltAfterEffects::SwitchState(state),
            AfterEffects::Weighted(routings) => {
                let weighted = WeightedIndex::new(routings.iter().map(|(x, _)| x)).unwrap();
                let routings = routings.iter().map(|(_, x)| *x).collect();
                BuiltAfterEffects::Weighted(weighted, routings)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum BuiltAfterEffects {
    Nothing,
    SwitchState(RoutingStateID),
    Weighted(WeightedIndex<Weight>, Vec<Option<RoutingStateID>>),
}

impl Distribution<Option<RoutingStateID>> for BuiltAfterEffects {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<RoutingStateID> {
        match self {
            BuiltAfterEffects::Nothing => None,
            BuiltAfterEffects::SwitchState(state) => Some(*state),
            BuiltAfterEffects::Weighted(weighted, stuff) => stuff[weighted.sample(rng)],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingState {
    pub after_click: AfterEffects,
    pub routings: BTreeMap<TrackID, (CompoundRoutingType, AfterEffects)>,
}

impl RoutingState {
    fn build(self) -> BuiltRoutingState {
        BuiltRoutingState {
            after_click: self.after_click.build(),
            routings: self
                .routings
                .into_iter()
                .map(|(track_id, (routing, ae))| (track_id, (routing.build(), ae.build())))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct BuiltRoutingState {
    after_click: BuiltAfterEffects,
    routings: BTreeMap<TrackID, (BuiltCompoundRoutingType, BuiltAfterEffects)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingInfo {
    pub default_state: RoutingStateID,
    pub states: BTreeMap<RoutingStateID, RoutingState>,
}

impl RoutingInfo {
    pub fn build(self) -> BuiltRouter {
        assert!(self.states.contains_key(&self.default_state));
        BuiltRouter {
            current_state: self.default_state,
            states: self
                .states
                .into_iter()
                .map(|(state_id, state)| (state_id, state.build()))
                .collect(),
        }
    }
}

pub struct BuiltRouter {
    current_state: RoutingStateID,
    states: BTreeMap<RoutingStateID, BuiltRoutingState>,
}

impl BuiltRouter {
    pub fn route<R: Rng + ?Sized>(&mut self, rng: &mut R, incoming: TrackID) -> RoutingType {
        let current_state = self.states.get(&self.current_state).unwrap();
        let outgoing = current_state.routings.get(&incoming).unwrap().0.sample(rng);
        if let Some(new_state) = current_state.routings.get(&incoming).unwrap().1.sample(rng) {
            self.current_state = new_state;
        }

        outgoing
    }

    pub fn clicked<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        let current_state = self.states.get(&self.current_state).unwrap();
        if let Some(new_state) = current_state.after_click.sample(rng) {
            self.current_state = new_state;
        }
    }
}
