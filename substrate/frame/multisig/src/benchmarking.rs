// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

// Benchmarks for Multisig Pallet

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame::benchmarking::prelude::*;

use crate::Pallet as Multisig;

const SEED: u32 = 0;

fn setup_multi<T: Config>(
	s: u32,
	z: u32,
) -> Result<(Vec<T::AccountId>, Box<<T as Config>::RuntimeCall>), &'static str> {
	let mut signatories: Vec<T::AccountId> = Vec::new();
	for i in 0..s {
		let signatory = account("signatory", i, SEED);
		// Give them some balance for a possible deposit
		let balance = BalanceOf::<T>::max_value();
		T::Currency::make_free_balance_be(&signatory, balance);
		signatories.push(signatory);
	}
	signatories.sort();
	// Must first convert to runtime call type.
	let call: <T as Config>::RuntimeCall =
		frame_system::Call::<T>::remark { remark: vec![0; z as usize] }.into();
	Ok((signatories, Box::new(call)))
}

#[benchmarks]
mod benchmarks {
	use super::*;

	/// `z`: Transaction Length
	#[benchmark]
	fn as_multi_threshold_1(z: Linear<0, 10_000>) -> Result<(), BenchmarkError> {
		let max_signatories = T::MaxSignatories::get().into();
		let (mut signatories, _) = setup_multi::<T>(max_signatories, z)?;
		let call: <T as Config>::RuntimeCall =
			frame_system::Call::<T>::remark { remark: vec![0; z as usize] }.into();
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), signatories, Box::new(call));

		// If the benchmark resolves, then the call was dispatched successfully.
		Ok(())
	}

	/// `z`: Transaction Length
	/// `s`: Signatories, need at least 2 people
	#[benchmark]
	fn as_multi_create(
		s: Linear<2, { T::MaxSignatories::get() }>,
		z: Linear<0, 10_000>,
	) -> Result<(), BenchmarkError> {
		let (mut signatories, call) = setup_multi::<T>(s, z)?;
		let call_hash = call.using_encoded(blake2_256);
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		as_multi(RawOrigin::Signed(caller), s as u16, signatories, None, call, Weight::zero());

		assert!(Multisigs::<T>::contains_key(multi_account_id, call_hash));

		Ok(())
	}

	/// `z`: Transaction Length
	/// `s`: Signatories, need at least 3 people (so we don't complete the multisig)
	#[benchmark]
	fn as_multi_approve(
		s: Linear<3, { T::MaxSignatories::get() }>,
		z: Linear<0, 10_000>,
	) -> Result<(), BenchmarkError> {
		let (mut signatories, call) = setup_multi::<T>(s, z)?;
		let call_hash = call.using_encoded(blake2_256);
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let mut signatories2 = signatories.clone();
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		// before the call, get the timepoint
		let timepoint = Multisig::<T>::timepoint();
		// Create the multi
		Multisig::<T>::as_multi(
			RawOrigin::Signed(caller).into(),
			s as u16,
			signatories,
			None,
			call.clone(),
			Weight::zero(),
		)?;
		let caller2 = signatories2.remove(0);
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller2);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		as_multi(
			RawOrigin::Signed(caller2),
			s as u16,
			signatories2,
			Some(timepoint),
			call,
			Weight::zero(),
		);

		let multisig =
			Multisigs::<T>::get(multi_account_id, call_hash).ok_or("multisig not created")?;
		assert_eq!(multisig.approvals.len(), 2);

		Ok(())
	}

	/// `z`: Transaction Length
	/// `s`: Signatories, need at least 2 people
	#[benchmark]
	fn as_multi_complete(
		s: Linear<2, { T::MaxSignatories::get() }>,
		z: Linear<0, 10_000>,
	) -> Result<(), BenchmarkError> {
		let (mut signatories, call) = setup_multi::<T>(s, z)?;
		let call_hash = call.using_encoded(blake2_256);
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let mut signatories2 = signatories.clone();
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		// before the call, get the timepoint
		let timepoint = Multisig::<T>::timepoint();
		// Create the multi
		Multisig::<T>::as_multi(
			RawOrigin::Signed(caller).into(),
			s as u16,
			signatories,
			None,
			call.clone(),
			Weight::zero(),
		)?;
		// Everyone except the first person approves
		for i in 1..s - 1 {
			let mut signatories_loop = signatories2.clone();
			let caller_loop = signatories_loop.remove(i as usize);
			let o = RawOrigin::Signed(caller_loop).into();
			Multisig::<T>::as_multi(
				o,
				s as u16,
				signatories_loop,
				Some(timepoint),
				call.clone(),
				Weight::zero(),
			)?;
		}
		let caller2 = signatories2.remove(0);
		assert!(Multisigs::<T>::contains_key(&multi_account_id, call_hash));
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller2);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		as_multi(
			RawOrigin::Signed(caller2),
			s as u16,
			signatories2,
			Some(timepoint),
			call,
			Weight::MAX,
		);

		assert!(!Multisigs::<T>::contains_key(&multi_account_id, call_hash));

		Ok(())
	}

	/// `s`: Signatories, need at least 2 people
	#[benchmark]
	fn approve_as_multi_create(
		s: Linear<2, { T::MaxSignatories::get() }>,
	) -> Result<(), BenchmarkError> {
		// The call is neither in storage or an argument, so just use any:
		let call_len = 10_000;
		let (mut signatories, call) = setup_multi::<T>(s, call_len)?;
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		let call_hash = call.using_encoded(blake2_256);
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller);
		add_to_whitelist(caller_key.into());

		// Create the multi
		#[extrinsic_call]
		approve_as_multi(
			RawOrigin::Signed(caller),
			s as u16,
			signatories,
			None,
			call_hash,
			Weight::zero(),
		);

		assert!(Multisigs::<T>::contains_key(multi_account_id, call_hash));

		Ok(())
	}

	/// `s`: Signatories, need at least 2 people
	#[benchmark]
	fn approve_as_multi_approve(
		s: Linear<2, { T::MaxSignatories::get() }>,
	) -> Result<(), BenchmarkError> {
		// The call is neither in storage or an argument, so just use any:
		let call_len = 10_000;
		let (mut signatories, call) = setup_multi::<T>(s, call_len)?;
		let mut signatories2 = signatories.clone();
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		let call_hash = call.using_encoded(blake2_256);
		// before the call, get the timepoint
		let timepoint = Multisig::<T>::timepoint();
		// Create the multi
		Multisig::<T>::as_multi(
			RawOrigin::Signed(caller).into(),
			s as u16,
			signatories,
			None,
			call,
			Weight::zero(),
		)?;
		let caller2 = signatories2.remove(0);
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller2);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		approve_as_multi(
			RawOrigin::Signed(caller2),
			s as u16,
			signatories2,
			Some(timepoint),
			call_hash,
			Weight::zero(),
		);

		let multisig =
			Multisigs::<T>::get(multi_account_id, call_hash).ok_or("multisig not created")?;
		assert_eq!(multisig.approvals.len(), 2);

		Ok(())
	}

	/// `s`: Signatories, need at least 2 people
	#[benchmark]
	fn cancel_as_multi(s: Linear<2, { T::MaxSignatories::get() }>) -> Result<(), BenchmarkError> {
		// The call is neither in storage or an argument, so just use any:
		let call_len = 10_000;
		let (mut signatories, call) = setup_multi::<T>(s, call_len)?;
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		let call_hash = call.using_encoded(blake2_256);
		let timepoint = Multisig::<T>::timepoint();
		// Create the multi
		let o = RawOrigin::Signed(caller.clone()).into();
		Multisig::<T>::as_multi(o, s as u16, signatories.clone(), None, call, Weight::zero())?;
		assert!(Multisigs::<T>::contains_key(&multi_account_id, call_hash));
		// Whitelist caller account from further DB operations.
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), s as u16, signatories, timepoint, call_hash);

		assert!(!Multisigs::<T>::contains_key(multi_account_id, call_hash));

		Ok(())
	}

	/// `s`: Signatories, need at least 2 people
	#[benchmark]
	fn poke_deposit(s: Linear<2, { T::MaxSignatories::get() }>) -> Result<(), BenchmarkError> {
		// The call is neither in storage or an argument, so just use any:
		let call_len = 10_000;
		let (mut signatories, call) = setup_multi::<T>(s, call_len)?;
		let multi_account_id = Multisig::<T>::multi_account_id(&signatories, s.try_into().unwrap());
		let caller = signatories.pop().ok_or("signatories should have len 2 or more")?;
		let call_hash = call.using_encoded(blake2_256);
		// Create the multi
		Multisig::<T>::as_multi(
			RawOrigin::Signed(caller.clone()).into(),
			s as u16,
			signatories.clone(),
			None,
			call,
			Weight::zero(),
		)?;

		// Get the current multisig data
		let multisig = Multisigs::<T>::get(multi_account_id.clone(), call_hash)
			.ok_or("multisig not created")?;
		// The original deposit
		let old_deposit = multisig.deposit;
		assert_eq!(T::Currency::reserved_balance(&caller), old_deposit);

		let additional_amount = 2u32.into();
		let new_deposit = old_deposit.saturating_add(additional_amount);

		// Reserve the additional amount from the caller's balance
		T::Currency::reserve(&caller, additional_amount)?;
		assert_eq!(T::Currency::reserved_balance(&caller), new_deposit);
		// Update the storage with the new deposit
		Multisigs::<T>::try_mutate(
			&multi_account_id,
			call_hash,
			|maybe_multisig| -> DispatchResult {
				let mut multisig = maybe_multisig.take().ok_or(Error::<T>::NotFound)?;
				multisig.deposit = new_deposit;
				*maybe_multisig = Some(multisig);
				Ok(())
			},
		)
		.map_err(|_| BenchmarkError::Stop("Mutating storage to change deposits failed"))?;
		// Check that the deposit was updated in storage
		let multisig = Multisigs::<T>::get(multi_account_id.clone(), call_hash)
			.ok_or("Multisig not created")?;
		assert_eq!(multisig.deposit, new_deposit);

		// Whitelist caller account
		let caller_key = frame_system::Account::<T>::hashed_key_for(&caller);
		add_to_whitelist(caller_key.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), s as u16, signatories, call_hash);

		let multisig = Multisigs::<T>::get(multi_account_id.clone(), call_hash)
			.ok_or("Multisig not created")?;
		assert_eq!(multisig.deposit, old_deposit);
		assert_eq!(T::Currency::reserved_balance(&caller), old_deposit);
		Ok(())
	}

	impl_benchmark_test_suite!(Multisig, crate::tests::new_test_ext(), crate::tests::Test);
}
