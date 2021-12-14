// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The crate's tests.

use super::*;
use crate::mock::*;
use assert_matches::assert_matches;
use codec::Decode;
use frame_support::{
	assert_noop, assert_ok,
	dispatch::{DispatchError::BadOrigin, RawOrigin},
	traits::Contains,
};
use pallet_balances::Error as BalancesError;

// TODO: Scheduler should re-use `None` items in its `Agenda`.

#[test]
fn params_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(ReferendumCount::<Test>::get(), 0);
		assert_eq!(Balances::free_balance(42), 0);
		assert_eq!(Balances::total_issuance(), 600);
	});
}

#[test]
fn basic_happy_path_works() {
	new_test_ext().execute_with(|| {
		// #1: submit
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			set_balance_proposal_hash(1),
			AtOrAfter::At(10),
		));
		assert_eq!(Balances::reserved_balance(&1), 2);
		assert_eq!(ReferendumCount::<Test>::get(), 1);
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));
		run_to(4);
		assert_eq!(DecidingCount::<Test>::get(0), 0);
		run_to(5);
		// #5: 4 blocks after submit - vote should now be deciding.
		assert_eq!(DecidingCount::<Test>::get(0), 1);
		run_to(6);
		// #6: Lots of ayes. Should now be confirming.
		set_tally(0, 100, 0);
		run_to(8);
		// #8: Should be confirmed & ended.
		assert_ok!(Referenda::refund_decision_deposit(Origin::signed(2), 0));
		run_to(11);
		// #9: Should not yet be enacted.
		assert_eq!(Balances::free_balance(&42), 0);
		run_to(12);
		// #10: Proposal should be executed.
		assert_eq!(Balances::free_balance(&42), 1);
	});
}

#[test]
fn confirming_then_fail_works() {
	new_test_ext().execute_with(|| {
	});
}

#[test]
fn confirming_then_reconfirming_works() {
	new_test_ext().execute_with(|| {
	});
}

fn is_waiting(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Ongoing(ReferendumStatus { deciding: None, .. }))
	)
}

fn is_deciding(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Ongoing(ReferendumStatus { deciding: Some(_), .. }))
	)
}

fn is_deciding_and_failing(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Ongoing(ReferendumStatus {
			deciding: Some(DecidingStatus { confirming: None, .. }),
			..
		}))
	)
}

fn is_confirming(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Ongoing(ReferendumStatus {
			deciding: Some(DecidingStatus { confirming: Some(_), .. }),
			..
		}))
	)
}

fn is_approved(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Approved(..))
	)
}

fn is_rejected(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Rejected(..))
	)
}

fn is_cancelled(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Cancelled(..))
	)
}

fn is_killed(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Killed(..))
	)
}

#[test]
fn queueing_works() {
	new_test_ext().execute_with(|| {
		// Submit a proposal into a track with a queue len of 1.
		assert_ok!(Referenda::submit(
			Origin::signed(5),
			RawOrigin::Root.into(),
			set_balance_proposal_hash(0),
			AtOrAfter::After(0),
		));
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(5), 0));

		run_to(2);

		// Submit 3 more proposals into the same queue.
		for i in 1..=4 {
			assert_ok!(Referenda::submit(
				Origin::signed(i),
				RawOrigin::Root.into(),
				set_balance_proposal_hash(i),
				AtOrAfter::After(0),
			));
			assert_ok!(Referenda::place_decision_deposit(Origin::signed(i), i as u32));
			// TODO: decision deposit after some initial votes with a non-highest voted coming first.
		}
		assert_eq!(ReferendumCount::<Test>::get(), 5);

		run_to(5);
		// One should be being decided.
		assert_eq!(DecidingCount::<Test>::get(0), 1);
		assert!(is_deciding_and_failing(0));

		// Vote to set order.
		set_tally(1, 1, 10);
		set_tally(2, 2, 20);
		set_tally(3, 3, 30);
		set_tally(4, 100, 0);
		println!("Agenda #6: {:?}", pallet_scheduler::Agenda::<Test>::get(6));
		run_to(6);
		println!("{:?}", Vec::<_>::from(TrackQueue::<Test>::get(0)));

		// Cancel the first.
		assert_ok!(Referenda::cancel(Origin::signed(4), 0));
		assert!(is_cancelled(0));

		// The other with the most approvals (#4) should be being decided.
		assert_eq!(DecidingCount::<Test>::get(0), 1);
		assert!(is_deciding(4));
		assert!(is_confirming(4));

		// Vote on the remaining two to change order.
		println!("Set tally #1");
		set_tally(1, 30, 31);
		println!("{:?}", Vec::<_>::from(TrackQueue::<Test>::get(0)));
		println!("Set tally #2");
		set_tally(2, 20, 20);
		println!("{:?}", Vec::<_>::from(TrackQueue::<Test>::get(0)));

		// Let confirmation period end.
		run_to(8);

		// #4 should have been confirmed.
		assert!(is_approved(4));
		// #1 (the one with the most approvals) should now be being decided.
		assert!(is_deciding(1));

		// Let it end unsuccessfully.
		run_to(12);
		assert!(is_rejected(1));

		// #2 should now be being decided. It will (barely) pass.
		assert!(is_deciding_and_failing(2));

		// #2 moves into confirming at the last moment with a 50% approval.
		run_to(16);
		assert!(is_confirming(2));

		// #2 gets approved.
		run_to(18);
		assert!(is_approved(2));
		assert!(is_deciding(3));
/*
		// Vote enough on #2 for it to go into confirming.
		set_tally(2, 100, 0);
		assert!(is_confirming(2));

		run_to(14);
		set_tally(2, 100, 100);
		assert!(is_deciding_and_failing(2));

		run_to(15);
		set_tally(2, 1000, 100);
		assert!(is_confirming(2));

		run_to(17);
*/
	});
}

// TODO: Confirm -> Unconfirm -> Reconfirm -> Pass
// TODO: (End) Confirm -> Unconfirm (overtime) -> Pass
// TODO: (End) Confirm -> Unconfirm (overtime) -> Fail

#[test]
fn kill_when_confirming_works() {
	new_test_ext().execute_with(|| {
	});
}

#[test]
fn auto_timeout_should_happen_with_nothing_but_submit() {
	new_test_ext().execute_with(|| {
		// #1: submit
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			set_balance_proposal_hash(1),
			AtOrAfter::At(20),
		));
		run_to(20);
		assert_matches!(
			ReferendumInfoFor::<Test>::get(0),
			Some(ReferendumInfo::Ongoing(..))
		);
		run_to(21);
		// #11: Timed out - ended.
		assert_matches!(
			ReferendumInfoFor::<Test>::get(0),
			Some(ReferendumInfo::TimedOut(21, _, None))
		);

	});
}

#[test]
fn tracks_are_distinguished() {
	new_test_ext().execute_with(|| {
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			set_balance_proposal_hash(1),
			AtOrAfter::At(10),
		));
		assert_ok!(Referenda::submit(
			Origin::signed(2),
			RawOrigin::None.into(),
			set_balance_proposal_hash(2),
			AtOrAfter::At(20),
		));

		assert_ok!(Referenda::place_decision_deposit(Origin::signed(3), 0));
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(4), 1));

		let mut i = ReferendumInfoFor::<Test>::iter().collect::<Vec<_>>();
		i.sort_by_key(|x| x.0);
		assert_eq!(
			i,
			vec![
				(
					0,
					ReferendumInfo::Ongoing(ReferendumStatus {
						track: 0,
						origin: OriginCaller::system(RawOrigin::Root),
						proposal_hash: set_balance_proposal_hash(1),
						enactment: AtOrAfter::At(10),
						submitted: 1,
						submission_deposit: Deposit { who: 1, amount: 2 },
						decision_deposit: Some(Deposit { who: 3, amount: 10 }),
						deciding: None,
						tally: Tally { ayes: 0, nays: 0 },
						ayes_in_queue: None,
						alarm: Some((5, (5, 0))),
					})
				),
				(
					1,
					ReferendumInfo::Ongoing(ReferendumStatus {
						track: 1,
						origin: OriginCaller::system(RawOrigin::None),
						proposal_hash: set_balance_proposal_hash(2),
						enactment: AtOrAfter::At(20),
						submitted: 1,
						submission_deposit: Deposit { who: 2, amount: 2 },
						decision_deposit: Some(Deposit { who: 4, amount: 1 }),
						deciding: None,
						tally: Tally { ayes: 0, nays: 0 },
						ayes_in_queue: None,
						alarm: Some((3, (3, 0))),
					})
				),
			]
		);
	});
}

#[test]
fn submit_errors_work() {
	new_test_ext().execute_with(|| {
		let h = set_balance_proposal_hash(1);
		// No track for Signed origins.
		assert_noop!(
			Referenda::submit(Origin::signed(1), RawOrigin::Signed(2).into(), h, AtOrAfter::At(10),),
			Error::<Test>::NoTrack
		);

		// No funds for deposit
		assert_noop!(
			Referenda::submit(Origin::signed(10), RawOrigin::Root.into(), h, AtOrAfter::At(10),),
			BalancesError::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn decision_deposit_errors_work() {
	new_test_ext().execute_with(|| {
		let e = Error::<Test>::NotOngoing;
		assert_noop!(Referenda::place_decision_deposit(Origin::signed(2), 0), e);

		let h = set_balance_proposal_hash(1);
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			h,
			AtOrAfter::At(10),
		));
		let e = BalancesError::<Test>::InsufficientBalance;
		assert_noop!(Referenda::place_decision_deposit(Origin::signed(10), 0), e);

		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));
		let e = Error::<Test>::HaveDeposit;
		assert_noop!(Referenda::place_decision_deposit(Origin::signed(2), 0), e);
	});
}

#[test]
fn refund_deposit_works() {
	new_test_ext().execute_with(|| {
		let e = Error::<Test>::BadReferendum;
		assert_noop!(Referenda::refund_decision_deposit(Origin::signed(1), 0), e);

		let h = set_balance_proposal_hash(1);
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			h,
			AtOrAfter::At(10),
		));
		let e = Error::<Test>::NoDeposit;
		assert_noop!(Referenda::refund_decision_deposit(Origin::signed(2), 0), e);

		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));
		let e = Error::<Test>::Unfinished;
		assert_noop!(Referenda::refund_decision_deposit(Origin::signed(3), 0), e);

		run_to(11);
		assert_ok!(Referenda::refund_decision_deposit(Origin::signed(3), 0));
	});
}

#[test]
fn cancel_works() {
	new_test_ext().execute_with(|| {
		let h = set_balance_proposal_hash(1);
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			h,
			AtOrAfter::At(10),
		));
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));

		run_to(8);
		assert_ok!(Referenda::cancel(Origin::signed(4), 0));
		assert_ok!(Referenda::refund_decision_deposit(Origin::signed(3), 0));
		assert_matches!(
			ReferendumInfoFor::<Test>::get(0).unwrap(),
			ReferendumInfo::Cancelled(8, Deposit { who: 1, amount: 2 }, None)
		);
	});
}

#[test]
fn cancel_errors_works() {
	new_test_ext().execute_with(|| {
		let h = set_balance_proposal_hash(1);
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			h,
			AtOrAfter::At(10),
		));
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));
		assert_noop!(Referenda::cancel(Origin::signed(1), 0), BadOrigin);

		run_to(11);
		assert_noop!(Referenda::cancel(Origin::signed(4), 0), Error::<Test>::NotOngoing);
	});
}

#[test]
fn kill_works() {
	new_test_ext().execute_with(|| {
		let h = set_balance_proposal_hash(1);
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			h,
			AtOrAfter::At(10),
		));
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));

		run_to(8);
		assert_ok!(Referenda::kill(Origin::root(), 0));
		let e = Error::<Test>::NoDeposit;
		assert_noop!(Referenda::refund_decision_deposit(Origin::signed(3), 0), e);
		assert_matches!(ReferendumInfoFor::<Test>::get(0).unwrap(), ReferendumInfo::Killed(8));
	});
}

#[test]
fn kill_errors_works() {
	new_test_ext().execute_with(|| {
		let h = set_balance_proposal_hash(1);
		assert_ok!(Referenda::submit(
			Origin::signed(1),
			RawOrigin::Root.into(),
			h,
			AtOrAfter::At(10),
		));
		assert_ok!(Referenda::place_decision_deposit(Origin::signed(2), 0));
		assert_noop!(Referenda::kill(Origin::signed(4), 0), BadOrigin);

		run_to(11);
		assert_noop!(Referenda::kill(Origin::root(), 0), Error::<Test>::NotOngoing);
	});
}

#[test]
fn set_balance_proposal_is_correctly_filtered_out() {
	for i in 0..10 {
		let call = crate::mock::Call::decode(&mut &set_balance_proposal(i)[..]).unwrap();
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
	}
}