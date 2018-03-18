// Linters
#![allow(dead_code)]
// Unstable features
#![feature(associated_type_defaults)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]

use std::marker::PhantomData;

////////////////
// Interfaces //
////////////////

pub trait StateContainer {
    type State;
}

pub trait State {
    type Transaction;
}

/* Markers */
pub trait Transaction {}
pub trait Service {}

pub trait TopLevelState: State {}
pub trait WaitableState: State {}

/* Functionality traits */
pub trait ServiceCompliance<S>
where
    S: Service,
{
    fn get(&self) -> &S;
    fn get_mut(&mut self) -> &mut S;
}

// Note: Awaiting trait aliases to do this fancy thing..
// pub trait StorageCompliant = ServiceCompliance<StorageService>;

pub trait TransitionFrom<T>
where
    T: StateContainer + Sized + 'static,
    Self: StateContainer + Sized + 'static,
    Self::State: State,
    <Self::State as State>::Transaction: Transaction + Copy,
{
    fn transition_from(_: T, _: <Self::State as State>::Transaction) -> Self;
}

pub trait PushdownFrom<T>
where
    T: StateContainer + Sized + 'static,
    Self: StateContainer + ServiceCompliance<StorageService> + Sized + 'static,
    Self::State: State,
    <Self::State as State>::Transaction: Transaction + Copy,
{
    fn pushdown_from(_: T, _: <Self::State as State>::Transaction) -> Self;
}

pub trait PullupFrom<T>
where
    T: StateContainer + ServiceCompliance<StorageService> + Sized + 'static,
    Self: Sized + StateContainer + 'static,
    Self::State: State,
{
    fn pullup_from(_: T) -> Self;
}

//////////////
// Services //
//////////////

#[derive(Debug)]
pub struct StorageService {
	// TODO
}
impl Service for StorageService {}

/////////////////////
// (State) Machine //
/////////////////////

#[derive(Debug)]
pub struct Machine<X>
where
    X: TopLevelState,
{
    pub state: PhantomData<X>,
    pub transaction: X::Transaction,
    pub storage: StorageService,
}

impl<X> StateContainer for Machine<X>
where
    X: TopLevelState,
{
    type State = X;
}

impl<X> ServiceCompliance<StorageService> for Machine<X>
where
    X: TopLevelState,
{
    fn get(&self) -> &StorageService {
        &self.storage
    }

    fn get_mut(&mut self) -> &mut StorageService {
        &mut self.storage
    }
}

///////////////////
// Toplevel WAIT //
///////////////////

#[derive(Debug)]
pub struct Wait<W: WaitableState>(W);
impl<W> State for Wait<W>
where
    W: WaitableState,
{
    type Transaction = W::Transaction;
}
impl<W> TopLevelState for Wait<W>
where
    W: WaitableState,
{
}

//
#[derive(Debug)]
pub struct Start();
impl State for Start {
    type Transaction = ();
}
impl WaitableState for Start {}

//
#[derive(Debug)]
pub struct Input();
impl State for Input {
    type Transaction = ();
}
impl WaitableState for Input {}

///////////////////////
// Toplevel FINISHED //
///////////////////////

#[derive(Debug)]
pub struct Finished();
impl State for Finished {
    type Transaction = ();
}
impl TopLevelState for Finished {}

////////////////////////////////
// Transition implementations //
////////////////////////////////

// Utility implementation for 0-sized transactions, the type `()`.
impl Transaction for () {}

/* Machine<WaitableState<Start>> -> Machine<WaitableState<Input>> */
impl TransitionFrom<Machine<Wait<Start>>> for Machine<Wait<Input>> {
    fn transition_from(old: Machine<Wait<Start>>, t: <Self::State as State>::Transaction) -> Self {
        Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        }
    }
}

/* Machine<WaitableState<Input>> -> Machine<Finished> */
impl TransitionFrom<Machine<Wait<Input>>> for Machine<Finished> {
    fn transition_from(old: Machine<Wait<Input>>, t: <Self::State as State>::Transaction) -> Self {
        Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        }
    }
}
