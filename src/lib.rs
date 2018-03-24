// Linters
#![allow(dead_code, let_and_return, unused_mut, unused_variables)]
// Unstable features
#![feature(associated_type_defaults)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate bincode;

use std::marker::PhantomData;
//
use serde::{Serialize, Deserialize};

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
pub trait ActionableState: State {}

// Type which can be used as empty transaction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Epsilon;
impl Transaction for Epsilon {}

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
    <Self::State as State>::Transaction: Transaction + Serialize + Deserialize<'static> + Copy + 'static,
{
    fn pushdown_from(_: T, _: <Self::State as State>::Transaction) -> Self;
}

pub trait PullupFrom<T>
where
    T: StateContainer + ServiceCompliance<StorageService> + Sized + 'static,
    Self: Sized + StateContainer + 'static,
    Self::State: State,
    <Self::State as State>::Transaction: Transaction + Deserialize<'static> + 'static,
{
    fn pullup_from(_: T) -> Self;
}

//////////////
// Services //
//////////////

#[derive(Debug)]
pub struct StorageService {
	// Used encoding for the tape:
    // [DATA-1] [LENGTH-1] [DATA-2] [LENGTH-2] ||WRITE_HEAD|| .. [EOF]
    // This provides a reverse linked list so we can easily push and pop data.
    pub tape: Vec<u8>
}
impl Service for StorageService {}

impl StorageService {
    pub fn write_tape<T: Serialize>(&mut self, t: T) -> bincode::Result<()> {
        unimplemented!()
    }

    pub fn read_tape<T: Deserialize<'static>>(&mut self) -> bincode::Result<T> {
        unimplemented!()
    }
}

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
    type Transaction = Epsilon;
}
impl WaitableState for Start {}

//
#[derive(Debug)]
pub struct Input();
impl State for Input {
    type Transaction = Epsilon;
}
impl WaitableState for Input {}

/////////////////////
// Toplevel ACTION //
/////////////////////

#[derive(Debug)]
pub struct Action<A: ActionableState>(A);
impl<A> State for Action<A> 
where
    A: ActionableState,
{
    type Transaction = A::Transaction;
}
impl<A> TopLevelState for Action<A> 
where
    A: ActionableState,
{
}

//
#[derive(Debug)]
pub struct Load();
impl State for Load {
    type Transaction = Epsilon;
}
impl ActionableState for Load {}

//
#[derive(Debug)]
pub struct Print();
impl State for Print {
    type Transaction = PrintTransaction;
}
impl ActionableState for Print {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PrintTransaction(pub &'static str);
impl Transaction for PrintTransaction {}

///////////////////////
// Toplevel FINISHED //
///////////////////////

#[derive(Debug)]
pub struct Finished();
impl State for Finished {
    type Transaction = Epsilon;
}
impl TopLevelState for Finished {}

////////////////////////////////
// Transition implementations //
////////////////////////////////

/* Machine<Wait<Start>> -> Machine<Wait<Input>> */
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

/* Machine<Wait<Input>> -> Machine<Finished> */
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

/* Machine<Wait<Input>> <-> Machine<Action<Print>> */
impl PushdownFrom<Machine<Wait<Input>>> for Machine<Action<Print>> {
    fn pushdown_from(old: Machine<Wait<Input>>, t: <Self::State as State>::Transaction) -> Self {
        // Build new machine.
        let mut new = Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        };

        // TODO: Archive state of the old machine.

        // Return new machine.
        new
    }
}

/* Machine<Wait<Input>> <-> Machine<Action<Print>> */
impl PullupFrom<Machine<Action<Print>>> for Machine<Wait<Input>> {
    fn pullup_from(mut old: Machine<Action<Print>>) -> Self {
        // TODO; Restore previously stored state.
        // DBG; Transaction shouldn't be declared here!
        let old_transaction = Epsilon;

        // Build new machine.
        Machine {
            state: PhantomData,
            transaction: old_transaction,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        }
    }
}

/* Machine<Action<Print>> <-> Machine<Action<Load>> */
impl PushdownFrom<Machine<Action<Print>>> for Machine<Action<Load>> {
    fn pushdown_from(old: Machine<Action<Print>>, t: <Self::State as State>::Transaction) -> Self {
        // Build new machine.
        let mut new = Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        };

        // TODO: Archive state of the old machine.

        // Return new machine.
        new
    }
}

/* Machine<Action<Print>> <-> Machine<Action<Load>> */
impl PullupFrom<Machine<Action<Load>>> for Machine<Action<Print>> {
    fn pullup_from(mut old: Machine<Action<Load>>) -> Self {
        // TODO; Restore previously stored state.
        // DBG; Transaction shouldn't be declared here!
        let old_transaction = PrintTransaction("DBG");

        // Build new machine.
        Machine {
            state: PhantomData,
            transaction: old_transaction,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        }
    }
}
