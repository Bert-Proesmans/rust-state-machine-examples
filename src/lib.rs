// Linters.
#![allow(dead_code, unused_mut, unused_variables, let_and_return, useless_format)]
// Prevent successful compilation when documentation is missing.
#![deny(missing_docs)]
// Unstable features.
#![feature(associated_type_defaults, universal_impl_trait, try_from, never_type)]
// Clippy linting when building debug versions.
#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(clippy))]
// Linters for code residing in documentation.
#![doc(test(attr(allow(unused_variables), deny(warnings))))]


//! Project intended for incremental building of a state machine.
//! The intention is to make every language item presented to the developer
//! as explicit as possible. While still allowing some degree of dynamic
//! flow.
//! Only using safe code of-course!

#[macro_use]
extern crate failure;

pub mod function {
    //! Contains the core functionality items for our system.
    use marker::Service;

    ////////////////
    // Interfaces //
    ////////////////

    /* Functionality traits */

    /// Trait generalizing over any structure that could act as a container of states.
    /// 
    /// This container of states could be reworded as 'the state machine' itself.
    pub trait StateContainer {
        /// Type of the current state held by the state machine.
        type State;
    }

    /// Trait generalizing over any state that's present in the state machine.
    pub trait State {
        /// Type of structure which must be provided when transitioning into the state
        /// represented by the enclosing type.
        type Transaction;
    }

    /// Trait for implementing a certain service on the state machine.
    /// 
    /// Because of this design exactly one object of each service type can be hooked onto 
    /// the same state machine.
    pub trait ServiceCompliance<S>
    where
        S: Service,
        Self: StateContainer
    {
        /// Retrieves an immutable reference to service `S`.
        fn get(&self) -> &S;
        /// Retrieves a mutable reference to service `S`.
        fn get_mut(&mut self) -> &mut S;
    }

    pub mod helper {
        //! Core functionality helper methods.
        //!
        //! Expect to find small utilities here, but they are mostly used by the hidden parts of the core.
        use std::convert::TryInto;

        use marker::{Transaction, TransactionContainer};

        /* Transaction helpers */
        /// Transform a transaction into the wrapping variant.
        pub fn pack_transaction<T, TC>(x: T) -> TC
        where
            T: Transaction + Into<TC> + Sized + 'static,
            TC: TransactionContainer + Sized + 'static,
        {
            x.into()
        }

        /// Unpack a wrapped transaction into an owned value.
        /// 
        /// It's of course necessary to 
        pub fn unpack_transaction<T, TC>(tc: TC) -> Result<T, TC::Error>
        where
            T: Transaction + Sized + 'static,
            TC: TransactionContainer + TryInto<T> + Sized + 'static,
        {
            tc.try_into()
        }
    }
}

pub mod marker {
    //! Primitive traits which can be used as constraints by the core components.
    //! 
    //! Marker Traits are usefull because the can be used as generic bounds. This allows
    //! for decoupling hidden code from developer created code.
    //! Correct understanding of what each trait encompasses is necessary!

    /// Types used to transition between state machine States.
    pub trait Transaction {}
    /// Types which generalize multiple transactions into 1 [`Sized`] structure
    /// so the transactions themselves can be safely stored in memory.
    pub trait TransactionContainer {}
    /// Types which attribute functionality to state machines.
    /// 
    /// A Service is kind-of like a Trait (language item), but is used in a dynamic
    /// way to quickly de-/construct state machines with various functional methods.
    pub trait Service {}

    /// (State) Types which are directly contained by the state machine.
    /// 
    /// Note: States can be nested!
    pub trait TopLevelMarker {}
    /// (State) Types which represent a condition for when the state machine itself
    /// should resume execution.
    /// 
    /// The semantics are limited to the set of input events a user can generate.
    pub trait WaitableMarker {}
    /// (State) Types which represent a condition for when the state machine itself
    /// should resume execution.
    /// 
    /// The semantics are limited to the set of action events a user can generate.
    pub trait ActionableMarker {}
}

pub mod stm {
    //! Traits enforcing state machine behaviour.
    use failure::Error;

    use function::{ServiceCompliance, State, StateContainer};
    use marker::{Transaction, TransactionContainer};
    use service::StackStorage;

    /// Types, state machines residing in a certain state, which transform one-sided
    /// into a next Type.
    /// 
    /// A state machine is said to transition from A into B when the current state is A,
    /// a Transaction object for state B is provided and the following transition is
    /// valid [A -> B].
    pub trait TransitionFrom<T>
    where
        T: StateContainer + Sized + 'static,
        Self: StateContainer + Sized + 'static,
        Self::State: State,
        <Self::State as State>::Transaction: Transaction + Copy,
    {
        /// Transition from the provided state into the implementing state.
        fn transition_from(_: T, _: <Self::State as State>::Transaction) -> Self;
    }

    /// Types, state machines residing in a certain state, which transform one-sided
    /// into a next Type. The Transaction object of the previous state is stored for re-use.
    /// 
    /// [`PushdownFrom`] is designed to be used together with [`PullupFrom`] because one part of 
    /// it's functionality is to store the previous state's Transaction onto a stack.
    /// Generally each [`PushDown`] must be followed with a matching Pullup operation to 
    /// correctly push onto and pop from the stackstorage.
    /// 
    /// A state machine is said to pushdown from A into B when the current state is A,
    /// a Transaction object for state B is provided and the following transition is
    /// valid [A -> B].
    pub trait PushdownFrom<T, TTC>
    where
        TTC: TransactionContainer + 'static,
        T: StateContainer + Sized + 'static,
        Self: StateContainer + ServiceCompliance<StackStorage<TTC>> + Sized + 'static,
        Self::State: State,
        <Self::State as State>::Transaction: Transaction + Copy + 'static,
    {
        /// Transition from the provided state into the implementing state.
        fn pushdown_from(_: T, _: <Self::State as State>::Transaction) -> Self;
    }

    /// Types, state machines residing in a certain state, which transform one-sided
    /// into a previous Type. The Transaction object of the next state is loaded 
    /// and restored.
    /// 
    /// [`PullupFrom`] is designed to be used together with [`PushdownFrom`] because one part of 
    /// it's functionality is to restore the next state's Transaction from a stack.
    /// Generally each [`PushDown`] must be followed with a matching Pullup operation to 
    /// correctly push onto and pop from the stackstorage.
    /// 
    /// A state machine is said to pullup from B into A when the current state is B
    /// and the following transition is valid [A <- B].
    pub trait PullupFrom<T, TTC>
    where
        TTC: TransactionContainer + 'static,
        T: StateContainer + ServiceCompliance<StackStorage<TTC>> + Sized + 'static,
        Self: Sized + StateContainer + 'static,
        Self::State: State,
        <Self::State as State>::Transaction: Transaction + 'static,
    {
        /// Transition from the provided state into the implementing state.
        /// 
        /// # Errors
        /// There is a check at runtime which prevents a Pullup transition if it doesn't match
        /// the correct PushDown transition in a First In, Last Out (FILO) manner.
        /// Note: This part CANNOT be statically verified as far as I know?
        fn pullup_from(_: T) -> Result<Self, Error>;
    }
}

pub mod service {
    //! Types which attribute functionality to state machines.
    use failure::Error;

    use marker::{Service, TransactionContainer};

    //////////////
    // Services //
    //////////////

    /// Structure wrapping a Vector type to provide a simple Stack interface.
    #[derive(Debug)]
    pub struct StackStorage<A>
    where
        A: TransactionContainer,
    {
        /// Backing storage for the emulated Stack functionality.
        pub tape: Vec<A>,
    }

    impl<A> Service for StackStorage<A>
    where
        A: TransactionContainer,
    {
    }

    impl<A> StackStorage<A>
    where
        A: TransactionContainer,
    {
        /// Add the provided value onto the top of the Stack.
        pub fn push<T: Into<A>>(&mut self, t: T) -> Result<(), !> {
            self.tape.push(t.into());
            Ok(())
        }

        /// Remove the element from the top of the Stack.
        /// 
        /// The popped value will match the value which was pushed last
        /// before executing this method.
        pub fn pop(&mut self) -> Result<A, Error> {
            self.tape
                .pop()
                .ok_or_else(|| format_err!("Popped too many!"))
        }
    }
}

pub mod state {
    //! Types which encode the states to be used by a state machine.

    use function::State;
    use marker::{ActionableMarker, TopLevelMarker, WaitableMarker};
    use transaction::{Epsilon, PrintTransaction};

    ///////////////////
    // Toplevel WAIT //
    ///////////////////

    /// State indicating a pause until an input event has been generated.
    #[derive(Debug)]
    pub struct Wait<W: WaitableMarker>(W);
    impl<W> State for Wait<W>
    where
        W: WaitableMarker + State,
    {
        type Transaction = W::Transaction;
    }

    impl<W> TopLevelMarker for Wait<W>
    where
        W: WaitableMarker,
    {
    }

    /// Wait condition state until the game has been started.
    #[derive(Debug)]
    pub struct Start();
    impl State for Start {
        type Transaction = Epsilon;
    }

    impl WaitableMarker for Start {}

    /// Wait condition state until the user has provided input.
    #[derive(Debug)]
    pub struct Input();
    impl State for Input {
        type Transaction = Epsilon;
    }

    impl WaitableMarker for Input {}

    /////////////////////
    // Toplevel ACTION //
    /////////////////////

    /// State indicating dynamic execution of the specific action is in progress.
    #[derive(Debug)]
    pub struct Action<A: ActionableMarker>(A);
    impl<A> State for Action<A>
    where
        A: ActionableMarker + State,
    {
        type Transaction = A::Transaction;
    }

    impl<A> TopLevelMarker for Action<A>
    where
        A: ActionableMarker,
    {
    }

    /// Action condition state indicating loading is in progress.
    #[derive(Debug)]
    pub struct Load();
    impl State for Load {
        type Transaction = Epsilon;
    }

    impl ActionableMarker for Load {}

    /// Action condition state indicating printing is in progress.
    #[derive(Debug)]
    pub struct Print();
    impl State for Print {
        // !-- See below *Transactions --!
        type Transaction = PrintTransaction;
    }

    impl ActionableMarker for Print {}

    ///////////////////////
    // Toplevel FINISHED //
    ///////////////////////

    /// State indicating finalization of the state machine.
    /// 
    /// Finished CAN NOT have any outgoing transitions, since it's intended
    /// to be a terminal state.
    #[derive(Debug)]
    pub struct Finished();
    impl State for Finished {
        type Transaction = Epsilon;
    }

    impl TopLevelMarker for Finished {}
}

pub mod transaction {
    //! Types used to convey transition related information.

    use std::convert::TryFrom;

    use failure::Error;

    use marker::{Transaction, TransactionContainer};

    //////////////////
    // Transactions //
    //////////////////

    /// Collection of known Transaction structures wrapped into a Sized
    /// item.
    #[derive(Debug)]
    pub enum TransactionItem {
        /// See [`Epsilon`]
        Epsilon(Epsilon),
        /// See [`PrintTransaction`]
        Print(PrintTransaction),
    }

    impl TransactionContainer for TransactionItem {}

    /// Empty Transaction object.
    /// 
    /// The name Epsilon is derived from NFA's where they indicate zero-step transitions
    /// between states.
    /// In this design it's intention is to convey that no Transition information is
    /// necessary to transition into a next state.
    #[derive(Debug, Clone, Copy)]
    pub struct Epsilon;
    impl Transaction for Epsilon {}

    impl From<Epsilon> for TransactionItem {
        fn from(x: Epsilon) -> Self {
            TransactionItem::Epsilon(x)
        }
    }

    impl TryFrom<TransactionItem> for Epsilon {
        type Error = Error;

        fn try_from(tc: TransactionItem) -> Result<Self, Self::Error> {
            match tc {
                TransactionItem::Epsilon(x) => Ok(x),
                _ => Err(format_err!("Unexpected item")),
            }
        }
    }

    /// Transaction to be received by states with printing behaviour.
    /// 
    /// This state is pure exemplary, I don't know what else to tell you
    /// about it..
    #[derive(Debug, Clone, Copy)]
    pub struct PrintTransaction(pub &'static str);
    impl Transaction for PrintTransaction {}

    impl From<PrintTransaction> for TransactionItem {
        fn from(x: PrintTransaction) -> Self {
            TransactionItem::Print(x)
        }
    }

    impl TryFrom<TransactionItem> for PrintTransaction {
        type Error = Error;

        fn try_from(tc: TransactionItem) -> Result<Self, Self::Error> {
            match tc {
                TransactionItem::Print(x) => Ok(x),
                _ => Err(format_err!("Unexpected item")),
            }
        }
    }
}

use std::marker::PhantomData;

use failure::Error;

use function::{ServiceCompliance, State, StateContainer};
use function::helper::{pack_transaction, unpack_transaction};
use marker::TopLevelMarker;
use service::StackStorage;
use stm::{PullupFrom, PushdownFrom, TransitionFrom};
use transaction::TransactionItem;
use state::*;

/////////////////////
// (State) Machine //
/////////////////////

/// The state machine.
/// 
/// The developer is encouraged to design this structure in any desired
/// way by storing services into it's members.
/// Each state machine MUST have a `state` and `transaction` field AT
/// MINIMUM.
#[derive(Debug)]
pub struct Machine<X>
where
    X: TopLevelMarker + State,
{
    /* Absolute minimum variables */
    /// Field to encode the current state of the machine.
    /// 
    /// This field is present to utilize the type system to statically verify
    /// legal transitions of the machine. This field has no (/zero) size 
    /// at runtime.
    pub state: PhantomData<X>,
    /// Field to store the provided Transaction object as rquired by the 
    /// current state.
    pub transaction: X::Transaction,

    /* Optionals */
    /// Stack storage service to allow PushDown and Pullup behaviour to be
    /// implemented.
    pub storage: StackStorage<TransactionItem>,
}

impl<X> StateContainer for Machine<X>
where
    X: TopLevelMarker + State,
{
    type State = X;
}

impl<X> ServiceCompliance<StackStorage<TransactionItem>> for Machine<X>
where
    X: TopLevelMarker + State,
{
    fn get(&self) -> &StackStorage<TransactionItem> {
        &self.storage
    }

    fn get_mut(&mut self) -> &mut StackStorage<TransactionItem> {
        &mut self.storage
    }
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
    fn pushdown_from(
        mut old: Machine<Wait<Input>>,
        t: <Self::State as State>::Transaction,
    ) -> Self {
        // Archive state of the old machine.
        let old_transaction: TransactionItem = pack_transaction(old.transaction);
        ServiceCompliance::<StackStorage<TransactionItem>>::get_mut(&mut old)
            .push(old_transaction)
            .expect("Never type triggered!");

        // Build new machine.
        Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        }
    }
}

/* Machine<Wait<Input>> <-> Machine<Action<Print>> */
impl PullupFrom<Machine<Action<Print>>, TransactionItem> for Machine<Wait<Input>> {
    fn pullup_from(mut old: Machine<Action<Print>>) -> Result<Self, Error> {
        // Restore previously stored state.
        let old_transaction = ServiceCompliance::<StackStorage<TransactionItem>>::get_mut(&mut old)
            .pop()
            .and_then(unpack_transaction)?;

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
    fn pushdown_from(
        mut old: Machine<Action<Print>>,
        t: <Self::State as State>::Transaction,
    ) -> Self {
        // Archive state of the old machine.
        let old_transaction: TransactionItem = pack_transaction(old.transaction);
        ServiceCompliance::<StackStorage<TransactionItem>>::get_mut(&mut old)
            .push(old_transaction)
            .expect("Never type triggered!");

        // Build new machine.
        Machine {
            state: PhantomData,
            transaction: t,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        }
    }
}

/* Machine<Action<Print>> <-> Machine<Action<Load>> */
impl PullupFrom<Machine<Action<Load>>, TransactionItem> for Machine<Action<Print>> {
    fn pullup_from(mut old: Machine<Action<Load>>) -> Result<Self, Error> {
        // Restore previously stored state.
        let old_transaction = ServiceCompliance::<StackStorage<TransactionItem>>::get_mut(&mut old)
            .pop()
            .and_then(unpack_transaction)?;

        // Build new machine.
        Ok(Machine {
            state: PhantomData,
            transaction: old_transaction,
            // Following properties MUST stay in sync with `Machine` !
            storage: old.storage,
        })
    }
}
