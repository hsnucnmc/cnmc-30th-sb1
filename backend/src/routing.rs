use packet::{Direction, TrackID};
use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type RoutingStateID = u32;
pub type Weight = f64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoutingType {
    Derail,
    BounceBack,
    Track((TrackID, Direction)),
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

#[derive(Debug, Clone, PartialEq)]
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
            AfterEffects::Weighted(effects) => {
                let weighted = WeightedIndex::new(effects.iter().map(|(x, _)| x)).unwrap();
                let effects = effects.iter().map(|(_, x)| *x).collect();
                BuiltAfterEffects::Weighted(weighted, effects)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    pub forward_routings: BTreeMap<TrackID, (CompoundRoutingType, AfterEffects)>,
    pub backward_routings: BTreeMap<TrackID, (CompoundRoutingType, AfterEffects)>,
}

impl RoutingState {
    fn build(self) -> BuiltRoutingState {
        BuiltRoutingState {
            after_click: self.after_click.build(),
            routings: self
                .forward_routings
                .into_iter()
                .map(|(incoming, (routing, ae))| {
                    (
                        (incoming, Direction::Forward),
                        (routing.build(), ae.build()),
                    )
                })
                .chain(
                    self.backward_routings
                        .into_iter()
                        .map(|(incoming, (routing, ae))| {
                            (
                                (incoming, Direction::Backward),
                                (routing.build(), ae.build()),
                            )
                        }),
                )
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct BuiltRoutingState {
    after_click: BuiltAfterEffects,
    routings: BTreeMap<(TrackID, Direction), (BuiltCompoundRoutingType, BuiltAfterEffects)>,
}

fn configured_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingInfo {
    #[serde(skip_serializing, default = "configured_default")]
    pub configured: bool,
    pub default_state: RoutingStateID,
    pub states: BTreeMap<RoutingStateID, RoutingState>,
}

impl Default for RoutingInfo {
    fn default() -> Self {
        Self {
            configured: false,
            default_state: 0,
            states: BTreeMap::new(),
        }
    }
}

impl RoutingInfo {
    pub fn check(&self) -> Result<(), &'static str> {
        if !self.states.contains_key(&self.default_state) {
            return Err("Default state isn't defined in states");
        }

        for (_, state) in &self.states {
            match &state.after_click {
                AfterEffects::Nothing => {}
                AfterEffects::SwitchState(new_state) => {
                    if !self.states.contains_key(&new_state) {
                        return Err("State after clicking isn't defined in states");
                    }
                }
                AfterEffects::Weighted(weighted) => {
                    if weighted.is_empty() {
                        return Err("Possibilities after clicking is a empty list");
                    }
                    if weighted.iter().map(|(weight, _)| weight).sum::<f64>() == 0f64 {
                        return Err("Total weights adds up to zero after clicking");
                    }

                    for (weight, effect) in weighted {
                        if *weight < 0f64 {
                            return Err("Probability of effect after clicking is negative");
                        }
                        if let Some(new_state) = effect {
                            if !self.states.contains_key(new_state) {
                                return Err("State after clicking isn't defined in states");
                            }
                        }
                    }
                }
            }

            for (_, (routing, effects)) in &state.forward_routings {
                match routing {
                    CompoundRoutingType::Simple(_) => {}
                    CompoundRoutingType::Weighted(weighted) => {
                        if weighted.is_empty() {
                            return Err("Possibilities of a routing option is a empty list");
                        }
                        if weighted.iter().map(|(weight, _)| weight).sum::<f64>() == 0f64 {
                            return Err("Total weights for a routing option adds up to zero");
                        }

                        for (weight, _) in weighted {
                            if *weight < 0f64 {
                                return Err(
                                    "Probability of effect of a routing option is negative",
                                );
                            }
                        }
                    }
                }
                match effects {
                    AfterEffects::Nothing => {}
                    AfterEffects::SwitchState(new_state) => {
                        if !self.states.contains_key(&new_state) {
                            return Err("State after using route isn't defined in states");
                        }
                    }
                    AfterEffects::Weighted(weighted) => {
                        if weighted.is_empty() {
                            return Err("Possibilities after using route is a empty list");
                        }
                        if weighted.iter().map(|(weight, _)| weight).sum::<f64>() == 0f64 {
                            return Err("Total weights adds up to zero after using route");
                        }

                        for (weight, effect) in weighted {
                            if *weight < 0f64 {
                                return Err("Probability of effect after using route is negative");
                            }
                            if let Some(new_state) = effect {
                                if !self.states.contains_key(new_state) {
                                    return Err("State after using route isn't defined in states");
                                }
                            }
                        }
                    }
                }
            }

            for (_, (routing, effects)) in &state.backward_routings {
                match routing {
                    CompoundRoutingType::Simple(_) => {}
                    CompoundRoutingType::Weighted(weighted) => {
                        if weighted.is_empty() {
                            return Err("Possibilities of a routing option is a empty list");
                        }
                        if weighted.iter().map(|(weight, _)| weight).sum::<f64>() == 0f64 {
                            return Err("Total weights for a routing option adds up to zero");
                        }

                        for (weight, _) in weighted {
                            if *weight < 0f64 {
                                return Err(
                                    "Probability of effect of a routing option is negative",
                                );
                            }
                        }
                    }
                }
                match effects {
                    AfterEffects::Nothing => {}
                    AfterEffects::SwitchState(new_state) => {
                        if !self.states.contains_key(&new_state) {
                            return Err("State after using route isn't defined in states");
                        }
                    }
                    AfterEffects::Weighted(weighted) => {
                        if weighted.is_empty() {
                            return Err("Possibilities after using route is a empty list");
                        }
                        if weighted.iter().map(|(weight, _)| weight).sum::<f64>() == 0f64 {
                            return Err("Total weights adds up to zero after using route");
                        }

                        for (weight, effect) in weighted {
                            if *weight < 0f64 {
                                return Err("Probability of effect after using route is negative");
                            }
                            if let Some(new_state) = effect {
                                if !self.states.contains_key(new_state) {
                                    return Err("State after using route isn't defined in states");
                                }
                            }
                        }
                    }
                }
            }
        }

        // this check is disabled since it's time consuming shouldn't be a real issue
        // possible incoming nodes == connected nodes >= outgoing nodes unless there's teleportation
        // thus every outgoing nodes should be contained in ingoing nodes
        // let connected = self.incoming();
        // for routing_type in self.outcomes() {
        //     match routing_type {
        //         RoutingType::Derail => {},
        //         RoutingType::BounceBack => {},
        //         RoutingType::Track(outcome) => {
        //             if !connected.contains(&outcome) {
        //                 return Err("There's node that's a poosible outcome which we doesn't know how to handle");
        //             }
        //         },
        //     }
        // }

        Ok(())
    }

    pub fn incoming(&self) -> BTreeSet<(TrackID, Direction)> {
        let mut incoming = BTreeSet::new();

        for (_, state) in &self.states {
            for (incoming_route, _) in &state.forward_routings {
                incoming.insert((*incoming_route, Direction::Forward));
            }

            for (incoming_route, _) in &state.backward_routings {
                incoming.insert((*incoming_route, Direction::Backward));
            }
        }

        incoming
    }

    pub fn outcomes(&self) -> BTreeSet<RoutingType> {
        let mut outcomes = BTreeSet::new();

        for (_, state) in &self.states {
            for (_, (routing, _)) in &state.forward_routings {
                match routing {
                    CompoundRoutingType::Simple(routing) => {
                        outcomes.insert(*routing);
                    }
                    CompoundRoutingType::Weighted(routings) => {
                        for (_, routing) in routings {
                            outcomes.insert(*routing);
                        }
                    }
                };
            }

            for (_, (routing, _)) in &state.backward_routings {
                match routing {
                    CompoundRoutingType::Simple(routing) => {
                        outcomes.insert(*routing);
                    }
                    CompoundRoutingType::Weighted(routings) => {
                        for (_, routing) in routings {
                            outcomes.insert(*routing);
                        }
                    }
                };
            }
        }

        outcomes
    }

    pub fn build(self) -> BuiltRouter {
        if self.configured {
            assert!(self.states.contains_key(&self.default_state));
        }

        BuiltRouter {
            configured: self.configured,
            current_state: self.default_state,
            states: self
                .states
                .into_iter()
                .map(|(state_id, state)| (state_id, state.build()))
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BuiltRouter {
    configured: bool,
    current_state: RoutingStateID,
    states: BTreeMap<RoutingStateID, BuiltRoutingState>,
}

impl BuiltRouter {
    pub fn state(&self) -> RoutingStateID {
        self.current_state
    }

    pub fn route<R: Rng + ?Sized>(
        &mut self,
        rng: &mut R,
        incoming: (TrackID, Direction),
    ) -> RoutingType {
        if !self.configured {
            return RoutingType::BounceBack;
        }

        let current_state = self.states.get(&self.current_state).unwrap();
        let outgoing = current_state.routings.get(&incoming).unwrap().0.sample(rng);
        if let Some(new_state) = current_state.routings.get(&incoming).unwrap().1.sample(rng) {
            self.current_state = new_state;
        }

        outgoing
    }

    pub fn clicked<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        if !self.configured {
            return;
        }
        let current_state = self.states.get(&self.current_state).unwrap();
        if let Some(new_state) = current_state.after_click.sample(rng) {
            self.current_state = new_state;
        }
    }
}
