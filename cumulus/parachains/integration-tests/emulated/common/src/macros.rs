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

pub use paste;

// Substrate
pub use frame_support::{pallet_prelude::Weight, weights::WeightToFee};
pub use pallet_assets;
pub use pallet_balances;
pub use pallet_message_queue;
pub use pallet_xcm;

// Polkadot
pub use polkadot_runtime_parachains::dmp::Pallet as Dmp;
pub use xcm::{
	prelude::{
		AccountId32, All, Asset, AssetId, BuyExecution, DepositAsset, ExpectTransactStatus,
		Fungible, Here, Location, MaybeErrorCode, OriginKind, RefundSurplus, Transact, Unlimited,
		VersionedAssets, VersionedXcm, WeightLimit, WithdrawAsset, Xcm,
	},
	v3::Location as V3Location,
};

// Cumulus
pub use asset_test_utils;
pub use cumulus_pallet_xcmp_queue;
pub use parachains_common::AccountId;
pub use xcm_emulator::Chain;

#[macro_export]
macro_rules! test_parachain_is_trusted_teleporter {
	( $sender_para:ty, $sender_xcm_config:ty, vec![$( $receiver_para:ty ),+], ($assets:expr, $amount:expr) ) => {
		$crate::macros::paste::paste! {
			// init Origin variables
			let sender = [<$sender_para Sender>]::get();
			let mut para_sender_balance_before =
				<$sender_para as $crate::macros::Chain>::account_data_of(sender.clone()).free;
			let origin = <$sender_para as $crate::macros::Chain>::RuntimeOrigin::signed(sender.clone());
			let fee_asset_item = 0;
			let weight_limit = $crate::macros::WeightLimit::Unlimited;

			$(
				{
					// init Destination variables
					let receiver = [<$receiver_para Receiver>]::get();
					let para_receiver_balance_before =
						<$receiver_para as $crate::macros::Chain>::account_data_of(receiver.clone()).free;
					let para_destination =
						<$sender_para>::sibling_location_of(<$receiver_para>::para_id());
					let beneficiary: Location =
						$crate::macros::AccountId32 { network: None, id: receiver.clone().into() }.into();

					// Send XCM message from Origin Parachain
					// We are only testing the limited teleport version, which should be ok since success will
					// depend only on a proper `XcmConfig` at destination.
					<$sender_para>::execute_with(|| {
						assert_ok!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::limited_teleport_assets(
							origin.clone(),
							bx!(para_destination.clone().into()),
							bx!(beneficiary.clone().into()),
							bx!($assets.clone().into()),
							fee_asset_item,
							weight_limit.clone(),
						));

						type RuntimeEvent = <$sender_para as $crate::macros::Chain>::RuntimeEvent;

						assert_expected_events!(
							$sender_para,
							vec![
								RuntimeEvent::PolkadotXcm(
									$crate::macros::pallet_xcm::Event::Attempted { outcome: Outcome::Complete { .. } }
								) => {},
								RuntimeEvent::XcmpQueue(
									$crate::macros::cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
								) => {},
								RuntimeEvent::Balances(
									$crate::macros::pallet_balances::Event::Burned { who: sender, amount }
								) => {},
							]
						);
					});

					// Receive XCM message in Destination Parachain
					<$receiver_para>::execute_with(|| {
						type RuntimeEvent = <$receiver_para as $crate::macros::Chain>::RuntimeEvent;

						assert_expected_events!(
							$receiver_para,
							vec![
								RuntimeEvent::Balances(
									$crate::macros::pallet_balances::Event::Minted { who: receiver, .. }
								) => {},
								RuntimeEvent::MessageQueue(
									$crate::macros::pallet_message_queue::Event::Processed { success: true, .. }
								) => {},
							]
						);
					});

					// Check if balances are updated accordingly in Origin and Destination Parachains
					let para_sender_balance_after =
						<$sender_para as $crate::macros::Chain>::account_data_of(sender.clone()).free;
					let para_receiver_balance_after =
						<$receiver_para as $crate::macros::Chain>::account_data_of(receiver.clone()).free;
					let delivery_fees = <$sender_para>::execute_with(|| {
						$crate::macros::asset_test_utils::xcm_helpers::teleport_assets_delivery_fees::<
							<$sender_xcm_config as xcm_executor::Config>::XcmSender,
						>($assets.clone(), fee_asset_item, weight_limit.clone(), beneficiary, para_destination)
					});

					assert_eq!(para_sender_balance_before - $amount - delivery_fees, para_sender_balance_after);
					assert!(para_receiver_balance_after > para_receiver_balance_before);

					// Update sender balance
					para_sender_balance_before = <$sender_para as $crate::macros::Chain>::account_data_of(sender.clone()).free;
				}
			)+
		}
	};
}

#[macro_export]
macro_rules! test_relay_is_trusted_teleporter {
	( $sender_relay:ty, $sender_xcm_config:ty, vec![$( $receiver_para:ty ),+], ($assets:expr, $amount:expr) ) => {
		$crate::macros::paste::paste! {
			// init Origin variables
			let sender = [<$sender_relay Sender>]::get();
			let mut relay_sender_balance_before =
				<$sender_relay as $crate::macros::Chain>::account_data_of(sender.clone()).free;
			let origin = <$sender_relay as $crate::macros::Chain>::RuntimeOrigin::signed(sender.clone());
			let fee_asset_item = 0;
			let weight_limit = $crate::macros::WeightLimit::Unlimited;

			$(
				{
					// init Destination variables
					let receiver = [<$receiver_para Receiver>]::get();
					let para_receiver_balance_before =
						<$receiver_para as $crate::macros::Chain>::account_data_of(receiver.clone()).free;
					let para_destination =
						<$sender_relay>::child_location_of(<$receiver_para>::para_id());
					let beneficiary: Location =
						$crate::macros::AccountId32 { network: None, id: receiver.clone().into() }.into();

					// Send XCM message from Relay
					<$sender_relay>::execute_with(|| {
						$crate::macros::Dmp::<<$sender_relay as $crate::macros::Chain>::Runtime>::make_parachain_reachable(<$receiver_para>::para_id());

						assert_ok!(<$sender_relay as [<$sender_relay Pallet>]>::XcmPallet::limited_teleport_assets(
							origin.clone(),
							bx!(para_destination.clone().into()),
							bx!(beneficiary.clone().into()),
							bx!($assets.clone().into()),
							fee_asset_item,
							weight_limit.clone(),
						));

						type RuntimeEvent = <$sender_relay as $crate::macros::Chain>::RuntimeEvent;

						assert_expected_events!(
							$sender_relay,
							vec![
								RuntimeEvent::XcmPallet(
									$crate::macros::pallet_xcm::Event::Attempted { outcome: Outcome::Complete { .. } }
								) => {},
								RuntimeEvent::Balances(
									$crate::macros::pallet_balances::Event::Burned { who: sender, amount }
								) => {},
								RuntimeEvent::XcmPallet(
									$crate::macros::pallet_xcm::Event::Sent { .. }
								) => {},
							]
						);
					});

					// Receive XCM message in Destination Parachain
					<$receiver_para>::execute_with(|| {
						type RuntimeEvent = <$receiver_para as $crate::macros::Chain>::RuntimeEvent;

						assert_expected_events!(
							$receiver_para,
							vec![
								RuntimeEvent::Balances(
									$crate::macros::pallet_balances::Event::Minted { who: receiver, .. }
								) => {},
								RuntimeEvent::MessageQueue(
									$crate::macros::pallet_message_queue::Event::Processed { success: true, .. }
								) => {},
							]
						);
					});

					// Check if balances are updated accordingly in Origin and Parachain
					let relay_sender_balance_after =
						<$sender_relay as $crate::macros::Chain>::account_data_of(sender.clone()).free;
					let para_receiver_balance_after =
						<$receiver_para as $crate::macros::Chain>::account_data_of(receiver.clone()).free;
					let delivery_fees = <$sender_relay>::execute_with(|| {
						$crate::macros::asset_test_utils::xcm_helpers::teleport_assets_delivery_fees::<
							<$sender_xcm_config as xcm_executor::Config>::XcmSender,
						>($assets.clone(), fee_asset_item, weight_limit.clone(), beneficiary, para_destination)
					});

					assert_eq!(relay_sender_balance_before - $amount - delivery_fees, relay_sender_balance_after);
					assert!(para_receiver_balance_after > para_receiver_balance_before);

					// Update sender balance
					relay_sender_balance_before = <$sender_relay as $crate::macros::Chain>::account_data_of(sender.clone()).free;
				}
			)+
		}
	};
}

#[macro_export]
macro_rules! test_parachain_is_trusted_teleporter_for_relay {
	( $sender_para:ty, $sender_xcm_config:ty, $receiver_relay:ty, $amount:expr ) => {
		$crate::macros::paste::paste! {
			// init Origin variables
			let sender = [<$sender_para Sender>]::get();
			let para_sender_balance_before =
				<$sender_para as $crate::macros::Chain>::account_data_of(sender.clone()).free;
			let origin = <$sender_para as $crate::macros::Chain>::RuntimeOrigin::signed(sender.clone());
			let assets: Assets = (Parent, $amount).into();
			let fee_asset_item = 0;
			let weight_limit = $crate::macros::WeightLimit::Unlimited;

			// init Destination variables
			let receiver = [<$receiver_relay Receiver>]::get();
			let relay_receiver_balance_before =
				<$receiver_relay as $crate::macros::Chain>::account_data_of(receiver.clone()).free;
			let relay_destination: Location = Parent.into();
			let beneficiary: Location =
				$crate::macros::AccountId32 { network: None, id: receiver.clone().into() }.into();

			// Send XCM message from Parachain
			<$sender_para>::execute_with(|| {
				assert_ok!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::limited_teleport_assets(
					origin.clone(),
					bx!(relay_destination.clone().into()),
					bx!(beneficiary.clone().into()),
					bx!(assets.clone().into()),
					fee_asset_item,
					weight_limit.clone(),
				));

				type RuntimeEvent = <$sender_para as $crate::macros::Chain>::RuntimeEvent;

				assert_expected_events!(
					$sender_para,
					vec![
						RuntimeEvent::PolkadotXcm(
							$crate::macros::pallet_xcm::Event::Attempted { outcome: Outcome::Complete { .. } }
						) => {},
						RuntimeEvent::Balances(
							$crate::macros::pallet_balances::Event::Burned { who: sender, amount }
						) => {},
						RuntimeEvent::PolkadotXcm(
							$crate::macros::pallet_xcm::Event::Sent { .. }
						) => {},
					]
				);
			});

			// Receive XCM message in Destination Parachain
			<$receiver_relay>::execute_with(|| {
				type RuntimeEvent = <$receiver_relay as $crate::macros::Chain>::RuntimeEvent;

				assert_expected_events!(
					$receiver_relay,
					vec![
						RuntimeEvent::Balances(
							$crate::macros::pallet_balances::Event::Minted { who: receiver, .. }
						) => {},
						RuntimeEvent::MessageQueue(
							$crate::macros::pallet_message_queue::Event::Processed { success: true, .. }
						) => {},
					]
				);
			});

			// Check if balances are updated accordingly in Origin and Relay Chain
			let para_sender_balance_after =
				<$sender_para as $crate::macros::Chain>::account_data_of(sender.clone()).free;
			let relay_receiver_balance_after =
				<$receiver_relay as $crate::macros::Chain>::account_data_of(receiver.clone()).free;
			let delivery_fees = <$sender_para>::execute_with(|| {
				$crate::macros::asset_test_utils::xcm_helpers::teleport_assets_delivery_fees::<
					<$sender_xcm_config as xcm_executor::Config>::XcmSender,
				>(assets, fee_asset_item, weight_limit.clone(), beneficiary, relay_destination)
			});

			assert_eq!(para_sender_balance_before - $amount - delivery_fees, para_sender_balance_after);
			assert!(relay_receiver_balance_after > relay_receiver_balance_before);
		}
	};
}

#[macro_export]
macro_rules! test_chain_can_claim_assets {
	( $sender_para:ty, $runtime_call:ty, $network_id:expr, $assets:expr, $amount:expr ) => {
		$crate::macros::paste::paste! {
			let sender = [<$sender_para Sender>]::get();
			let origin = <$sender_para as $crate::macros::Chain>::RuntimeOrigin::signed(sender.clone());
			// Receiver is the same as sender
			let beneficiary: Location =
				$crate::macros::AccountId32 { network: Some($network_id), id: sender.clone().into() }.into();
			let versioned_assets: $crate::macros::VersionedAssets = $assets.clone().into();

			<$sender_para>::execute_with(|| {
				// Assets are trapped for whatever reason.
				// The possible reasons for this might differ from runtime to runtime, so here we just drop them directly.
				<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::drop_assets(
					&beneficiary,
					$assets.clone().into(),
					&XcmContext { origin: None, message_id: [0u8; 32], topic: None },
				);

				type RuntimeEvent = <$sender_para as $crate::macros::Chain>::RuntimeEvent;
				assert_expected_events!(
					$sender_para,
					vec![
						RuntimeEvent::PolkadotXcm(
							$crate::macros::pallet_xcm::Event::AssetsTrapped { origin: beneficiary, assets: versioned_assets, .. }
						) => {},
					]
				);

				let balance_before = <$sender_para as [<$sender_para Pallet>]>::Balances::free_balance(&sender);

				// Different origin or different assets won't work.
				let other_origin = <$sender_para as $crate::macros::Chain>::RuntimeOrigin::signed([<$sender_para Receiver>]::get());
				assert!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::claim_assets(
					other_origin,
					bx!(versioned_assets.clone().into()),
					bx!(beneficiary.clone().into()),
				).is_err());
				let other_versioned_assets: $crate::macros::VersionedAssets = Assets::new().into();
				assert!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::claim_assets(
					origin.clone(),
					bx!(other_versioned_assets.into()),
					bx!(beneficiary.clone().into()),
				).is_err());

				// Assets will be claimed to `beneficiary`, which is the same as `sender`.
				assert_ok!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::claim_assets(
					origin.clone(),
					bx!(versioned_assets.clone().into()),
					bx!(beneficiary.clone().into()),
				));

				assert_expected_events!(
					$sender_para,
					vec![
						RuntimeEvent::PolkadotXcm(
							$crate::macros::pallet_xcm::Event::AssetsClaimed { origin: beneficiary, assets: versioned_assets, .. }
						) => {},
					]
				);

				// After claiming the assets, the balance has increased.
				let balance_after = <$sender_para as [<$sender_para Pallet>]>::Balances::free_balance(&sender);
				assert_eq!(balance_after, balance_before + $amount);

				// Claiming the assets again doesn't work.
				assert!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::claim_assets(
					origin.clone(),
					bx!(versioned_assets.clone().into()),
					bx!(beneficiary.clone().into()),
				).is_err());

				let balance = <$sender_para as [<$sender_para Pallet>]>::Balances::free_balance(&sender);
				assert_eq!(balance, balance_after);

				// You can also claim assets and send them to a different account.
				<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::drop_assets(
					&beneficiary,
					$assets.clone().into(),
					&XcmContext { origin: None, message_id: [0u8; 32], topic: None },
				);
				let receiver = [<$sender_para Receiver>]::get();
				let other_beneficiary: Location =
					$crate::macros::AccountId32 { network: Some($network_id), id: receiver.clone().into() }.into();
				let balance_before = <$sender_para as [<$sender_para Pallet>]>::Balances::free_balance(&receiver);
				assert_ok!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::claim_assets(
					origin.clone(),
					bx!(versioned_assets.clone().into()),
					bx!(other_beneficiary.clone().into()),
				));
				let balance_after = <$sender_para as [<$sender_para Pallet>]>::Balances::free_balance(&receiver);
				assert_eq!(balance_after, balance_before + $amount);
			});
		}
	};
}

#[macro_export]
macro_rules! test_can_estimate_and_pay_exact_fees {
	( $sender_para:ty, $asset_hub:ty, $receiver_para:ty, ($asset_id:expr, $amount:expr), $owner_prefix:ty ) => {
		$crate::macros::paste::paste! {
			// We first define the call we'll use throughout the test.
			fn get_call(
				estimated_local_fees: impl Into<Asset>,
				estimated_intermediate_fees: impl Into<Asset>,
				estimated_remote_fees: impl Into<Asset>,
			) -> <$sender_para as Chain>::RuntimeCall {
				type RuntimeCall = <$sender_para as Chain>::RuntimeCall;

				let beneficiary = [<$receiver_para Receiver>]::get();
				let xcm_in_destination = Xcm::<()>::builder_unsafe()
					.pay_fees(estimated_remote_fees)
					.deposit_asset(AllCounted(1), beneficiary)
					.build();
				let ah_to_receiver = $asset_hub::sibling_location_of($receiver_para::para_id());
				let xcm_in_reserve = Xcm::<()>::builder_unsafe()
					.pay_fees(estimated_intermediate_fees)
					.deposit_reserve_asset(
						AllCounted(1),
						ah_to_receiver,
						xcm_in_destination,
					)
					.build();
				let sender_to_ah = $sender_para::sibling_location_of($asset_hub::para_id());
				let local_xcm = Xcm::<<$sender_para as Chain>::RuntimeCall>::builder()
					.withdraw_asset(($asset_id, $amount))
					.pay_fees(estimated_local_fees)
					.initiate_reserve_withdraw(AllCounted(1), sender_to_ah, xcm_in_reserve)
					.build();

				RuntimeCall::PolkadotXcm(pallet_xcm::Call::execute {
					message: bx!(VersionedXcm::from(local_xcm)),
					max_weight: Weight::from_parts(10_000_000_000, 500_000),
				})
			}

			let destination = $sender_para::sibling_location_of($receiver_para::para_id());
			let sender = [<$sender_para Sender>]::get();
			let sender_as_seen_by_ah = $asset_hub::sibling_location_of($sender_para::para_id());
			let sov_of_sender_on_ah = $asset_hub::sovereign_account_id_of(sender_as_seen_by_ah.clone());
			let asset_owner = [<$owner_prefix AssetOwner>]::get();

			// Fund parachain's sender account.
			$sender_para::mint_foreign_asset(
				<$sender_para as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
				$asset_id.clone().into(),
				sender.clone(),
				$amount * 2,
			);

			// Fund the parachain origin's SA on Asset Hub with the native tokens.
			$asset_hub::fund_accounts(vec![(sov_of_sender_on_ah.clone(), $amount * 2)]);

			let beneficiary_id = [<$receiver_para Receiver>]::get();

			let test_args = TestContext {
				sender: sender.clone(),
				receiver: beneficiary_id.clone(),
				args: TestArgs::new_para(
					destination,
					beneficiary_id.clone(),
					$amount,
					($asset_id, $amount).into(),
					None,
					0,
				),
			};
			let mut test = ParaToParaThroughAHTest::new(test_args);

			// We get these from the closure.
			let mut local_execution_fees = 0;
			let mut local_delivery_fees = 0;
			let mut remote_message = VersionedXcm::from(Xcm::<()>(Vec::new()));
			<$sender_para as TestExt>::execute_with(|| {
				type Runtime = <$sender_para as Chain>::Runtime;
				type OriginCaller = <$sender_para as Chain>::OriginCaller;

				let call = get_call(
					(Parent, 100_000_000_000u128),
					(Parent, 100_000_000_000u128),
					(Parent, 100_000_000_000u128),
				);
				let origin = OriginCaller::system(RawOrigin::Signed(sender.clone()));
				let result = Runtime::dry_run_call(origin, call, xcm::prelude::XCM_VERSION).unwrap();
				let local_xcm = result.local_xcm.unwrap().clone();
				let local_xcm_weight = Runtime::query_xcm_weight(local_xcm).unwrap();
				local_execution_fees = Runtime::query_weight_to_asset_fee(
					local_xcm_weight,
					VersionedAssetId::from(AssetId(Location::parent())),
				)
				.unwrap();
				// We filter the result to get only the messages we are interested in.
				let (destination_to_query, messages_to_query) = &result
					.forwarded_xcms
					.iter()
					.find(|(destination, _)| {
						*destination == VersionedLocation::from(Location::new(1, [Parachain(1000)]))
					})
					.unwrap();
				assert_eq!(messages_to_query.len(), 1);
				remote_message = messages_to_query[0].clone();
				let delivery_fees =
					Runtime::query_delivery_fees(destination_to_query.clone(), remote_message.clone())
						.unwrap();
				local_delivery_fees = $crate::xcm_helpers::get_amount_from_versioned_assets(delivery_fees);
			});

			// These are set in the AssetHub closure.
			let mut intermediate_execution_fees = 0;
			let mut intermediate_delivery_fees = 0;
			let mut intermediate_remote_message = VersionedXcm::from(Xcm::<()>(Vec::new()));
			<$asset_hub as TestExt>::execute_with(|| {
				type Runtime = <$asset_hub as Chain>::Runtime;
				type RuntimeCall = <$asset_hub as Chain>::RuntimeCall;

				// First we get the execution fees.
				let weight = Runtime::query_xcm_weight(remote_message.clone()).unwrap();
				intermediate_execution_fees = Runtime::query_weight_to_asset_fee(
					weight,
					VersionedAssetId::from(AssetId(Location::new(1, []))),
				)
				.unwrap();

				// We have to do this to turn `VersionedXcm<()>` into `VersionedXcm<RuntimeCall>`.
				let xcm_program =
					VersionedXcm::from(Xcm::<RuntimeCall>::from(remote_message.clone().try_into().unwrap()));

				// Now we get the delivery fees to the final destination.
				let result =
					Runtime::dry_run_xcm(sender_as_seen_by_ah.clone().into(), xcm_program).unwrap();
				let (destination_to_query, messages_to_query) = &result
					.forwarded_xcms
					.iter()
					.find(|(destination, _)| {
						*destination == VersionedLocation::from(Location::new(1, [Parachain(2001)]))
					})
					.unwrap();
				// There's actually two messages here.
				// One created when the message we sent from `$sender_para` arrived and was executed.
				// The second one when we dry-run the xcm.
				// We could've gotten the message from the queue without having to dry-run, but
				// offchain applications would have to dry-run, so we do it here as well.
				intermediate_remote_message = messages_to_query[0].clone();
				let delivery_fees = Runtime::query_delivery_fees(
					destination_to_query.clone(),
					intermediate_remote_message.clone(),
				)
				.unwrap();
				intermediate_delivery_fees = $crate::xcm_helpers::get_amount_from_versioned_assets(delivery_fees);
			});

			// Get the final execution fees in the destination.
			let mut final_execution_fees = 0;
			<$receiver_para as TestExt>::execute_with(|| {
				type Runtime = <$sender_para as Chain>::Runtime;

				let weight = Runtime::query_xcm_weight(intermediate_remote_message.clone()).unwrap();
				final_execution_fees =
					Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::from(AssetId(Location::parent())))
						.unwrap();
			});

			// Dry-running is done.
			$sender_para::reset_ext();
			$asset_hub::reset_ext();
			$receiver_para::reset_ext();

			// Fund accounts again.
			$sender_para::mint_foreign_asset(
				<$sender_para as Chain>::RuntimeOrigin::signed(asset_owner),
				$asset_id.clone().into(),
				sender.clone(),
				$amount * 2,
			);
			$asset_hub::fund_accounts(vec![(sov_of_sender_on_ah, $amount * 2)]);

			// Actually run the extrinsic.
			let sender_assets_before = $sender_para::execute_with(|| {
				type ForeignAssets = <$sender_para as [<$sender_para Pallet>]>::ForeignAssets;
				<ForeignAssets as Inspect<_>>::balance($asset_id.clone().into(), &sender)
			});
			let receiver_assets_before = $receiver_para::execute_with(|| {
				type ForeignAssets = <$receiver_para as [<$receiver_para Pallet>]>::ForeignAssets;
				<ForeignAssets as Inspect<_>>::balance($asset_id.clone().into(), &beneficiary_id)
			});

			test.set_assertion::<$sender_para>(sender_assertions);
			test.set_assertion::<$asset_hub>(hop_assertions);
			test.set_assertion::<$receiver_para>(receiver_assertions);
			let call = get_call(
				(Parent, local_execution_fees + local_delivery_fees),
				(Parent, intermediate_execution_fees + intermediate_delivery_fees),
				(Parent, final_execution_fees),
			);
			test.set_call(call);
			test.assert();

			let sender_assets_after = $sender_para::execute_with(|| {
				type ForeignAssets = <$sender_para as [<$sender_para Pallet>]>::ForeignAssets;
				<ForeignAssets as Inspect<_>>::balance($asset_id.clone().into(), &sender)
			});
			let receiver_assets_after = $receiver_para::execute_with(|| {
				type ForeignAssets = <$receiver_para as [<$receiver_para Pallet>]>::ForeignAssets;
				<ForeignAssets as Inspect<_>>::balance($asset_id.into(), &beneficiary_id)
			});

			// We know the exact fees on every hop.
			assert_eq!(sender_assets_after, sender_assets_before - $amount);
			assert_eq!(
				receiver_assets_after,
				receiver_assets_before + $amount -
					local_execution_fees -
					local_delivery_fees -
					intermediate_execution_fees -
					intermediate_delivery_fees -
					final_execution_fees
			);
		}
	};
}

#[macro_export]
macro_rules! test_dry_run_transfer_across_pk_bridge {
	( $sender_asset_hub:ty, $sender_bridge_hub:ty, $destination:expr ) => {
		$crate::macros::paste::paste! {
			use frame_support::{dispatch::RawOrigin, traits::fungible};
			use sp_runtime::AccountId32;
			use xcm::prelude::*;
			use xcm_runtime_apis::dry_run::runtime_decl_for_dry_run_api::DryRunApiV2;

			let who = AccountId32::new([1u8; 32]);
			let transfer_amount = 10_000_000_000_000u128;
			let initial_balance = transfer_amount * 10;

			// AssetHub setup.
			$sender_asset_hub::force_xcm_version($destination, XCM_VERSION);

			<$sender_asset_hub as TestExt>::execute_with(|| {
				type Runtime = <$sender_asset_hub as Chain>::Runtime;
				type RuntimeCall = <$sender_asset_hub as Chain>::RuntimeCall;
				type OriginCaller = <$sender_asset_hub as Chain>::OriginCaller;
				type Balances = <$sender_asset_hub as [<$sender_asset_hub Pallet>]>::Balances;

				// Give some initial funds.
				<Balances as fungible::Mutate<_>>::set_balance(&who, initial_balance);

				let call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::transfer_assets {
					dest: Box::new(VersionedLocation::from($destination)),
					beneficiary: Box::new(VersionedLocation::from(Junction::AccountId32 {
						id: who.clone().into(),
						network: None,
					})),
					assets: Box::new(VersionedAssets::from(vec![
						(Parent, transfer_amount).into(),
					])),
					fee_asset_item: 0,
					weight_limit: Unlimited,
				});
				let result = Runtime::dry_run_call(OriginCaller::system(RawOrigin::Signed(who)), call, XCM_VERSION).unwrap();
				// We assert the dry run succeeds and sends only one message to the local bridge hub.
				assert!(result.execution_result.is_ok());
				assert_eq!(result.forwarded_xcms.len(), 1);
				assert_eq!(result.forwarded_xcms[0].0, VersionedLocation::from(Location::new(1, [Parachain($sender_bridge_hub::para_id().into())])));
			});
		}
	};
}

#[macro_export]
macro_rules! test_xcm_fee_querying_apis_work_for_asset_hub {
	( $asset_hub:ty ) => {
		$crate::macros::paste::paste! {
			use emulated_integration_tests_common::USDT_ID;
			use xcm_runtime_apis::fees::{Error as XcmPaymentApiError, runtime_decl_for_xcm_payment_api::XcmPaymentApiV1};

			$asset_hub::execute_with(|| {
				// Setup a pool between USDT and WND.
				type RuntimeOrigin = <$asset_hub as Chain>::RuntimeOrigin;
				type Assets = <$asset_hub as [<$asset_hub Pallet>]>::Assets;
				type AssetConversion = <$asset_hub as [<$asset_hub Pallet>]>::AssetConversion;
				let wnd = Location::new(1, []);
				let usdt = Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())]);
				let sender = [<$asset_hub Sender>]::get();
				assert_ok!(AssetConversion::create_pool(
					RuntimeOrigin::signed(sender.clone()),
					Box::new(wnd.clone()),
					Box::new(usdt.clone()),
				));

				type Runtime = <$asset_hub as Chain>::Runtime;
				let acceptable_payment_assets = Runtime::query_acceptable_payment_assets(XCM_VERSION).unwrap();
				assert_eq!(acceptable_payment_assets, vec![
					VersionedAssetId::from(AssetId(wnd.clone())),
					VersionedAssetId::from(AssetId(usdt.clone())),
				]);

				let program = Xcm::<()>::builder()
					.withdraw_asset((Parent, 100u128))
					.buy_execution((Parent, 10u128), Unlimited)
					.deposit_asset(All, [0u8; 32])
					.build();
				let weight = Runtime::query_xcm_weight(VersionedXcm::from(program)).unwrap();
				let fee_in_wnd = Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::from(AssetId(wnd.clone()))).unwrap();
				// Assets not in a pool don't work.
				assert!(Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::from(AssetId(Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(1)])))).is_err());
				let fee_in_usdt_fail = Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::from(AssetId(usdt.clone())));
				// Weight to asset fee fails because there's not enough asset in the pool.
				// We just created it, there's none.
				assert_eq!(fee_in_usdt_fail, Err(XcmPaymentApiError::AssetNotFound));
				// We add some.
				assert_ok!(Assets::mint(
					RuntimeOrigin::signed(sender.clone()),
					USDT_ID.into(),
					sender.clone().into(),
					5_000_000_000_000
				));
				// We make 1 WND = 4 USDT.
				assert_ok!(AssetConversion::add_liquidity(
					RuntimeOrigin::signed(sender.clone()),
					Box::new(wnd),
					Box::new(usdt.clone()),
					1_000_000_000_000,
					4_000_000_000_000,
					0,
					0,
					sender.into()
				));
				// Now it works.
				let fee_in_usdt = Runtime::query_weight_to_asset_fee(weight, VersionedAssetId::from(AssetId(usdt)));
				assert_ok!(fee_in_usdt);
				assert!(fee_in_usdt.unwrap() > fee_in_wnd);
			});
		}
	};
}

#[macro_export]
macro_rules! test_cross_chain_alias {
	( vec![$( ($sender_para:ty, $receiver_para:ty, $is_teleport:expr, $expected_success:expr) ),+], $origin:expr, $target:expr, $fees:expr ) => {
		$crate::macros::paste::paste! {
			$(
				{
					use xcm::latest::AssetTransferFilter;
					let para_destination = <$sender_para>::sibling_location_of(<$receiver_para>::para_id());
					let account: AccountId = $origin.clone().into();
					$sender_para::fund_accounts(vec![(account.clone(), $fees * 10)]);
					let total_fees: Asset = (Location::parent(), $fees).into();
					let fees: Asset = (Location::parent(), $fees / 2).into();

					let remote_fees = if $is_teleport {
						Some(AssetTransferFilter::Teleport(fees.clone().into()))
					} else {
						let source_para_sa = <$receiver_para>::sovereign_account_id_of(
							<$receiver_para>::sibling_location_of(<$sender_para>::para_id()),
						);
						$receiver_para::fund_accounts(vec![(source_para_sa, $fees * 10)]);
						Some(AssetTransferFilter::ReserveWithdraw(fees.clone().into()))
					};
					<$sender_para>::execute_with(|| {
						type RuntimeEvent = <$sender_para as $crate::macros::Chain>::RuntimeEvent;
						let xcm_message = Xcm::<()>(vec![
							WithdrawAsset(total_fees.into()),
							PayFees { asset: fees.clone() },
							InitiateTransfer {
								destination: para_destination,
								remote_fees,
								preserve_origin: true,
								assets: BoundedVec::new(),
								remote_xcm: Xcm(vec![
									// try to alias into `account`
									AliasOrigin($target.clone().into()),
									RefundSurplus,
									DepositAsset {
										assets: Wild(AllCounted(1)),
										beneficiary: $target.clone().into(),
									},
								]),
							},
							RefundSurplus,
							DepositAsset { assets: Wild(AllCounted(1)), beneficiary: account.clone().into() },
						]);

						let signed_origin = <$sender_para as Chain>::RuntimeOrigin::signed(account.into());
						assert_ok!(<$sender_para as [<$sender_para Pallet>]>::PolkadotXcm::execute(
							signed_origin,
							bx!(xcm::VersionedXcm::from(xcm_message.into())),
							Weight::MAX
						));
						assert_expected_events!(
							$sender_para,
							vec![
								RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
							]
						);
					});

					<$receiver_para>::execute_with(|| {
						type RuntimeEvent = <$receiver_para as $crate::macros::Chain>::RuntimeEvent;
						assert_expected_events!(
							$receiver_para,
							vec![
								RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
									success, ..
								}) => { success: *success == $expected_success, },
							]
						);
					});
				}
			)+
		}
	};
}

/// note: $asset needs to be prefunded outside this function
#[macro_export]
macro_rules! create_pool_with_native_on {
	( $chain:ident, $asset:expr, $is_foreign:expr, $asset_owner:expr ) => {
		$crate::create_pool_with_native_on!(
			$chain,
			$asset,
			$is_foreign,
			$asset_owner,
			1_000_000_000_000,
			2_000_000_000_000
		);
	};

	( $chain:ident, $asset:expr, $is_foreign:expr, $asset_owner:expr, $native_amount:expr, $asset_amount:expr ) => {
		emulated_integration_tests_common::impls::paste::paste! {
			<$chain>::execute_with(|| {
				type RuntimeEvent = <$chain as Chain>::RuntimeEvent;
				let owner = $asset_owner;
				let signed_owner = <$chain as Chain>::RuntimeOrigin::signed(owner.clone());
				let native_asset: Location = Parent.into();

				if $is_foreign {
					assert_ok!(<$chain as [<$chain Pallet>]>::ForeignAssets::mint(
						signed_owner.clone(),
						$asset.clone().into(),
						owner.clone().into(),
						10_000_000_000_000, // For it to have more than enough.
					));
				} else {
					let asset_id = match $asset.interior.last() {
						Some(GeneralIndex(id)) => *id as u32,
						_ => unreachable!(),
					};
					assert_ok!(<$chain as [<$chain Pallet>]>::Assets::mint(
						signed_owner.clone(),
						asset_id.into(),
						owner.clone().into(),
						10_000_000_000_000, // For it to have more than enough.
					));
				}

				assert_ok!(<$chain as [<$chain Pallet>]>::AssetConversion::create_pool(
					signed_owner.clone(),
					Box::new(native_asset.clone()),
					Box::new($asset.clone()),
				));

				assert_expected_events!(
					$chain,
					vec![
						RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
					]
				);

				assert_ok!(<$chain as [<$chain Pallet>]>::AssetConversion::add_liquidity(
					signed_owner,
					Box::new(native_asset),
					Box::new($asset),
					$native_amount,
					$asset_amount,
					0,
					0,
					owner.into()
				));

				assert_expected_events!(
					$chain,
					vec![
						RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded { .. }) => {},
					]
				);
			});
		}
	};
}
