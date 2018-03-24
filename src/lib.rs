// Linters
#![allow(dead_code, unused_mut, unused_variables, let_and_return)]
// Unstable features
#![feature(associated_type_defaults, universal_impl_trait, try_from)]
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]

use std::marker::PhantomData;
use std::convert::{TryFrom, TryInto};

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
pub trait TransactionContainer {}

pub trait Service {}

pub trait TopLevelState: State {}
pub trait WaitableState: State {}
pub trait ActionableState: State {}

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

pub trait PushdownFrom<T, TTC>
where
    TTC: TransactionContainer + 'static,
    T: StateContainer + Sized + 'static,
    Self: StateContainer + ServiceCompliance<StorageService<TTC>> + Sized + 'static,
    Self::State: State,
    <Self::State as State>::Transaction: Transaction + Copy + 'static,
{
    fn pushdown_from(_: T, _: <Self::State as State>::Transaction) -> Self;
}

pub trait PullupFrom<T, TTC>
where
    TTC: TransactionContainer + 'static,
    T: StateContainer + ServiceCompliance<StorageService<TTC>> + Sized + 'static,
    Self: Sized + StateContainer + 'static,
    Self::State: State,
    <Self::State as State>::Transaction: Transaction + 'static,
{
    fn pullup_from(_: T) -> Result<Self, String>;
}

//////////////
// Services //
//////////////

#[derive(Debug)]
pub struct StorageService<A>
where
    A: TransactionContainer,
{
    pub tape: Vec<A>,
}
impl<A> Service for StorageService<A>
where
    A: TransactionContainer,
{
}

impl<A> StorageService<A>
where
    A: TransactionContainer,
{
    pub fn write_tape<T: Into<A>>(&mut self, t: T) {
        self.tape.push(t.into());
    }

    pub fn read_tape<T>(&mut self) -> Result<T, String>
    where
        A: TryInto<T, Error = String>,
    {
        // Note; Written out explicitly because compiler couldn't infer involved types
        let item: Result<A, String> = self.tape.pop().ok_or_else(|| "Popped too many!".into());
        item?.try_into()
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
    pub storage: StorageService<TransactionItem>,
}

impl<X> StateContainer for Machine<X>
where
    X: TopLevelState,
{
    type State = X;
}

impl<X> ServiceCompliance<StorageService<TransactionItem>> for Machine<X>
where
    X: TopLevelState,
{
    fn get(&self) -> &StorageService<TransactionItem> {
        &self.storage
    }

    fn get_mut(&mut self) -> &mut StorageService<TransactionItem> {
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
    // !-- See below *Transactions --!
    type Transaction = PrintTransaction;
}
impl ActionableState for Print {}

///////////////////////
// Toplevel FINISHED //
///////////////////////

#[derive(Debug)]
pub struct Finished();
impl State for Finished {
    type Transaction = Epsilon;
}
impl TopLevelState for Finished {}

//////////////////
// Transactions //
//////////////////

#[derive(Debug)]
pub enum TransactionItem {
    Epsilon(Epsilon),
    Print(PrintTransaction),
}
impl TransactionContainer for TransactionItem {}

// Type which can be used as empty transaction.
#[derive(Debug, Clone, Copy)]
pub struct Epsilon;
impl Transaction for Epsilon {}

impl From<Epsilon> for TransactionItem {
    fn from(x: Epsilon) -> Self {
        TransactionItem::Epsilon(x)
    }
}

impl TryFrom<TransactionItem> for Epsilon {
    type Error = String;

    fn try_from(tc: TransactionItem) -> Result<Self, Self::Error> {
        match tc {
            TransactionItem::Epsilon(x) => Ok(x),
            _ => Err("Unexpected item".into()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrintTransaction(pub &'static str);
impl Transaction for PrintTransaction {}

impl From<PrintTransaction> for TransactionItem {
    fn from(x: PrintTransaction) -> Self {
        TransactionItem::Print(x)
    }
}

impl TryFrom<TransactionItem> for PrintTransaction {
    type Error = String;

    fn try_from(tc: TransactionItem) -> Result<Self, Self::Error> {
        match tc {
            TransactionItem::Print(x) => Ok(x),
            _ => Err("Unexpected item".into()),
        }
    }
}

/* Transaction helpers */
pub fn pack_transaction<T, TC>(x: T) -> TC
where
    T: Transaction + Into<TC> + Sized + 'static,
    TC: TransactionContainer + Sized + 'static,
{
    x.into()
}

pub fn unpack_transaction<T, TC>(tc: TC) -> Result<T, TC::Error>
where
    T: Transaction + Sized + 'static,
    TC: TransactionContainer + TryInto<T> + Sized + 'static,
{
    tc.try_into()
}

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
impl PushdownFrom<Machine<Wait<Input>>, TransactionItem> for Machine<Action<Print>> {
    fn pushdown_from(old: Machine<Wait<Input>>, t: <Self::State as State>::Transaction) -> Self {
        // Build new machine.
        let mut new = Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        };

        // TODO: Archive state of the old machine.
        new.storage.write_tape(old.transaction);

        // Return new machine.
        new
    }
}

/* Machine<Wait<Input>> <-> Machine<Action<Print>> */
impl PullupFrom<Machine<Action<Print>>, TransactionItem> for Machine<Wait<Input>> {
    fn pullup_from(mut old: Machine<Action<Print>>) -> Result<Self, String> {
        // TODO; Restore previously stored state.
        // DBG; Transaction shouldn't be declared here!
        // let old_transaction = Epsilon;
        let old_transaction = old.storage.read_tape()?;

        // Build new machine.
        Ok(Machine {
            state: PhantomData,
            transaction: old_transaction,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        })
    }
}

/* Machine<Action<Print>> <-> Machine<Action<Load>> */
impl PushdownFrom<Machine<Action<Print>>, TransactionItem> for Machine<Action<Load>> {
    fn pushdown_from(old: Machine<Action<Print>>, t: <Self::State as State>::Transaction) -> Self {
        // Build new machine.
        let mut new = Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        };

        // TODO: Archive state of the old machine.
        new.storage.write_tape(old.transaction);

        // Return new machine.
        new
    }
}

/* Machine<Action<Print>> <-> Machine<Action<Load>> */
impl PullupFrom<Machine<Action<Load>>, TransactionItem> for Machine<Action<Print>> {
    fn pullup_from(mut old: Machine<Action<Load>>) -> Result<Self, String> {
        // TODO; Restore previously stored state.
        // DBG; Transaction shouldn't be declared here!
        // let old_transaction = PrintTransaction("DBG");
        let old_transaction = old.storage.read_tape()?;

        // Build new machine.
        Ok(Machine {
            state: PhantomData,
            transaction: old_transaction,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        })
    }
}
