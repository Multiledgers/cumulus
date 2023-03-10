// Copyright (C) 2023 Parity Technologies (UK) Ltd.
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

use crate::{
	AccountIdOf, BalanceOf, ExtBuilder, RuntimeHelper, SessionKeysOf, ValidatorIdOf,
	XcmReceivedFrom,
};
use assets_common::AssetIdForTrustBackedAssetsConvert;
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok, pallet_prelude::Get, traits::OriginTrait, weights::Weight,
};
use parachains_common::{AssetIdForTrustBackedAssets, Balance};
use sp_runtime::traits::{StaticLookup, Zero};
use xcm::latest::prelude::*;
use xcm_executor::{traits::Convert, XcmExecutor};

pub struct CollatorSessionKeys<
	Runtime: frame_system::Config + pallet_balances::Config + pallet_session::Config,
> {
	collator: AccountIdOf<Runtime>,
	validator: ValidatorIdOf<Runtime>,
	key: SessionKeysOf<Runtime>,
}

impl<Runtime: frame_system::Config + pallet_balances::Config + pallet_session::Config>
	CollatorSessionKeys<Runtime>
{
	pub fn new(
		collator: AccountIdOf<Runtime>,
		validator: ValidatorIdOf<Runtime>,
		key: SessionKeysOf<Runtime>,
	) -> Self {
		Self { collator, validator, key }
	}
	pub fn collators(&self) -> Vec<AccountIdOf<Runtime>> {
		vec![self.collator.clone()]
	}

	pub fn session_keys(
		&self,
	) -> Vec<(AccountIdOf<Runtime>, ValidatorIdOf<Runtime>, SessionKeysOf<Runtime>)> {
		vec![(self.collator.clone(), self.validator.clone(), self.key.clone())]
	}
}

/// Test-case makes sure that `Runtime` can receive teleported native assets from relay chain
pub fn receive_teleported_asset_for_native_asset_works<Runtime, XcmConfig>(
	collator_session_keys: CollatorSessionKeys<Runtime>,
	target_account: AccountIdOf<Runtime>,
) where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_collator_selection::Config
		+ cumulus_pallet_parachain_system::Config,
	AccountIdOf<Runtime>: Into<[u8; 32]>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	BalanceOf<Runtime>: From<Balance>,
	XcmConfig: xcm_executor::Config,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys.collators())
		.with_session_keys(collator_session_keys.session_keys())
		.build()
		.execute_with(|| {
			// check Balances before
			assert_eq!(<pallet_balances::Pallet<Runtime>>::free_balance(&target_account), 0.into());

			let native_asset_id = MultiLocation::parent();

			let xcm = Xcm(vec![
				ReceiveTeleportedAsset(MultiAssets::from(vec![MultiAsset {
					id: Concrete(native_asset_id),
					fun: Fungible(10000000000000),
				}])),
				ClearOrigin,
				BuyExecution {
					fees: MultiAsset {
						id: Concrete(native_asset_id),
						fun: Fungible(10000000000000),
					},
					weight_limit: Limited(Weight::from_parts(303531000, 65536)),
				},
				DepositAsset {
					assets: Wild(AllCounted(1)),
					beneficiary: MultiLocation {
						parents: 0,
						interior: X1(AccountId32 {
							network: None,
							id: target_account.clone().into(),
						}),
					},
				},
			]);

			let hash = xcm.using_encoded(sp_io::hashing::blake2_256);

			let outcome = XcmExecutor::<XcmConfig>::execute_xcm(
				Parent,
				xcm,
				hash,
				RuntimeHelper::<Runtime>::xcm_max_weight(XcmReceivedFrom::Parent),
			);
			assert_eq!(outcome.ensure_complete(), Ok(()));

			// check Balances after
			assert_ne!(<pallet_balances::Pallet<Runtime>>::free_balance(&target_account), 0.into());
		})
}

#[macro_export]
macro_rules! include_receive_teleported_asset_for_native_asset_works(
	(
		$runtime:path,
		$xcm_config:path,
		$collator_session_key:expr
	) => {
		#[test]
		fn receive_teleported_asset_for_native_asset_works() {
			const BOB: [u8; 32] = [2u8; 32];
			let target_account = parachains_common::AccountId::from(BOB);

			asset_test_utils::test_cases::receive_teleported_asset_for_native_asset_works::<
				$runtime,
				$xcm_config
			>($collator_session_key, target_account)
		}
	}
);

/// Test-case makes sure that `Runtime` can receive teleported assets from sibling parachain relay chain
pub fn receive_teleported_asset_from_foreign_creator_works<
	Runtime,
	XcmConfig,
	WeightToWee,
	SovereignAccountOf,
	ForeignAssetsPalletInstance,
>(
	collator_session_keys: CollatorSessionKeys<Runtime>,
	target_account: AccountIdOf<Runtime>,
	existential_deposit: BalanceOf<Runtime>,
	asset_owner: AccountIdOf<Runtime>,
) where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_collator_selection::Config
		+ cumulus_pallet_parachain_system::Config
		+ pallet_assets::Config<ForeignAssetsPalletInstance>,
	AccountIdOf<Runtime>: Into<[u8; 32]>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	BalanceOf<Runtime>: From<Balance>,
	XcmConfig: xcm_executor::Config,
	WeightToWee: frame_support::weights::WeightToFee<Balance = Balance>,
	<WeightToWee as frame_support::weights::WeightToFee>::Balance: From<u128> + Into<u128>,
	SovereignAccountOf: Convert<MultiLocation, AccountIdOf<Runtime>>,
	<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::AssetId:
		From<MultiLocation> + Into<MultiLocation>,
	<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::AssetIdParameter:
		From<MultiLocation> + Into<MultiLocation>,
	<Runtime as pallet_assets::Config<ForeignAssetsPalletInstance>>::Balance:
		From<Balance> + Into<u128>,
	<Runtime as frame_system::Config>::AccountId:
		Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
	<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
		From<<Runtime as frame_system::Config>::AccountId>,
	ForeignAssetsPalletInstance: 'static,
{
	// foreign parachain with the same consenus currency as asset
	let foreign_asset_id_multilocation =
		MultiLocation { parents: 1, interior: X2(Parachain(2222), GeneralIndex(1234567)) };

	// foreign creator, which can be sibling parachain to match ForeignCreators
	let foreign_creator = MultiLocation { parents: 1, interior: X1(Parachain(2222)) };
	let foreign_creator_as_account_id = SovereignAccountOf::convert(foreign_creator).expect("");

	// we want to buy execution with local relay chain currency
	let buy_execution_fee_amount =
		WeightToWee::weight_to_fee(&Weight::from_parts(90_000_000_000, 0));
	let buy_execution_fee = MultiAsset {
		id: Concrete(MultiLocation::parent()),
		fun: Fungible(buy_execution_fee_amount.into()),
	};

	let teleported_foreign_asset_amount = 10000000000000;

	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys.collators())
		.with_session_keys(collator_session_keys.session_keys())
		.with_balances(vec![
			(
				foreign_creator_as_account_id.clone(),
				existential_deposit + (buy_execution_fee_amount * 2).into(),
			),
			(target_account.clone(), existential_deposit),
		])
		.with_tracing()
		.build()
		.execute_with(|| {
			// checks before
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&target_account),
				existential_deposit
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
					foreign_asset_id_multilocation.into(),
					&target_account
				),
				0.into()
			);

			// create foreign asset
			let asset_minimum_asset_balance = 3333333_u128;
			assert_ok!(
				<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::force_create(
					RuntimeHelper::<Runtime>::root_origin(),
					foreign_asset_id_multilocation.clone().into(),
					asset_owner.into(),
					false,
					asset_minimum_asset_balance.into()
				)
			);
			assert!(teleported_foreign_asset_amount > asset_minimum_asset_balance);

			// prepare xcm
			let xcm = Xcm(vec![
				// BuyExecution with relaychain native token
				WithdrawAsset(buy_execution_fee.clone().into()),
				BuyExecution {
					fees: MultiAsset {
						id: Concrete(MultiLocation::parent()),
						fun: Fungible(buy_execution_fee_amount.into()),
					},
					weight_limit: Limited(Weight::from_parts(403531000, 1024)),
				},
				// Process teleported asset
				ReceiveTeleportedAsset(MultiAssets::from(vec![MultiAsset {
					id: Concrete(foreign_asset_id_multilocation),
					fun: Fungible(teleported_foreign_asset_amount),
				}])),
				DepositAsset {
					assets: Wild(AllOf {
						id: Concrete(foreign_asset_id_multilocation),
						fun: WildFungibility::Fungible,
					}),
					beneficiary: MultiLocation {
						parents: 0,
						interior: X1(AccountId32 {
							network: None,
							id: target_account.clone().into(),
						}),
					},
				},
			]);
			let hash = xcm.using_encoded(sp_io::hashing::blake2_256);

			let outcome = XcmExecutor::<XcmConfig>::execute_xcm(
				foreign_creator,
				xcm,
				hash,
				RuntimeHelper::<Runtime>::xcm_max_weight(XcmReceivedFrom::Sibling),
			);
			assert_eq!(outcome.ensure_complete(), Ok(()));

			// checks after
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&target_account),
				existential_deposit
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, ForeignAssetsPalletInstance>>::balance(
					foreign_asset_id_multilocation.into(),
					&target_account
				),
				teleported_foreign_asset_amount.into()
			);
		})
}

#[macro_export]
macro_rules! include_receive_teleported_asset_from_foreign_creator_works(
	(
		$runtime:path,
		$xcm_config:path,
		$weight_to_fee:path,
		$sovereign_account_of:path,
		$assets_pallet_instance:path,
		$collator_session_key:expr,
		$existential_deposit:expr
	) => {
		#[test]
		fn receive_teleported_asset_from_foreign_creator_works() {
			const BOB: [u8; 32] = [2u8; 32];
			let target_account = parachains_common::AccountId::from(BOB);
			const SOME_ASSET_OWNER: [u8; 32] = [5u8; 32];
			let asset_owner = parachains_common::AccountId::from(SOME_ASSET_OWNER);

			asset_test_utils::test_cases::receive_teleported_asset_from_foreign_creator_works::<
				$runtime,
				$xcm_config,
				$weight_to_fee,
				$sovereign_account_of,
				$assets_pallet_instance
			>($collator_session_key, target_account, $existential_deposit, asset_owner)
		}
	}
);

/// Test-case makes sure that `Runtime`'s `xcm::AssetTransactor` can handle native relay chain currency
pub fn asset_transactor_transfer_with_local_consensus_currency_works<Runtime, XcmConfig>(
	collator_session_keys: CollatorSessionKeys<Runtime>,
	source_account: AccountIdOf<Runtime>,
	target_account: AccountIdOf<Runtime>,
	existential_deposit: BalanceOf<Runtime>,
	additional_checks_before: Box<dyn Fn()>,
	additional_checks_after: Box<dyn Fn()>,
) where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_collator_selection::Config
		+ cumulus_pallet_parachain_system::Config,
	AccountIdOf<Runtime>: Into<[u8; 32]>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	BalanceOf<Runtime>: From<Balance>,
	XcmConfig: xcm_executor::Config,
	<Runtime as pallet_balances::Config>::Balance: From<Balance> + Into<u128>,
	<Runtime as frame_system::Config>::AccountId:
		Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
	<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
		From<<Runtime as frame_system::Config>::AccountId>,
{
	let unit = existential_deposit;

	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys.collators())
		.with_session_keys(collator_session_keys.session_keys())
		.with_balances(vec![(source_account.clone(), (BalanceOf::<Runtime>::from(10_u128) * unit))])
		.with_tracing()
		.build()
		.execute_with(|| {
			// check Balances before
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(source_account.clone()),
				(BalanceOf::<Runtime>::from(10_u128) * unit)
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(target_account.clone()),
				(BalanceOf::<Runtime>::zero() * unit)
			);

			// additional check before
			additional_checks_before();

			// transfer_asset (deposit/withdraw) ALICE -> BOB
			let _ = RuntimeHelper::<XcmConfig>::do_transfer(
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: source_account.clone().into() }),
				},
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: target_account.clone().into() }),
				},
				// local_consensus_currency_asset, e.g.: relaychain token (KSM, DOT, ...)
				(
					MultiLocation { parents: 1, interior: Here },
					(BalanceOf::<Runtime>::from(1_u128) * unit).into(),
				),
			)
			.expect("no error");

			// check Balances after
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(source_account),
				(BalanceOf::<Runtime>::from(9_u128) * unit)
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(target_account),
				(BalanceOf::<Runtime>::from(1_u128) * unit)
			);

			// additional check after
			additional_checks_after();
		})
}

#[macro_export]
macro_rules! include_asset_transactor_transfer_with_local_consensus_currency_works(
	(
		$runtime:path,
		$xcm_config:path,
		$collator_session_key:expr,
		$existential_deposit:expr,
		$additional_checks_before:expr,
		$additional_checks_after:expr
	) => {
		#[test]
		fn asset_transactor_transfer_with_local_consensus_currency_works() {
			const ALICE: [u8; 32] = [1u8; 32];
			let source_account = parachains_common::AccountId::from(ALICE);
			const BOB: [u8; 32] = [2u8; 32];
			let target_account = parachains_common::AccountId::from(BOB);

			asset_test_utils::test_cases::asset_transactor_transfer_with_local_consensus_currency_works::<
				$runtime,
				$xcm_config
			>(
				$collator_session_key,
				source_account,
				target_account,
				$existential_deposit,
				$additional_checks_before,
				$additional_checks_after
			)
		}
	}
);

///Test-case makes sure that `Runtime`'s `xcm::AssetTransactor` can handle native relay chain currency
pub fn asset_transactor_transfer_with_trust_backed_assets_works<
	Runtime,
	XcmConfig,
	AssetsPalletInstance,
	TrustBackedAssetsPalletLocation,
>(
	collator_session_keys: CollatorSessionKeys<Runtime>,
	existential_deposit: BalanceOf<Runtime>,
	asset_deposit: BalanceOf<Runtime>,
	asset_owner: AccountIdOf<Runtime>,
	alice_account: AccountIdOf<Runtime>,
	bob_account: AccountIdOf<Runtime>,
	charlie_account: AccountIdOf<Runtime>,
	additional_checks_before: Box<dyn Fn()>,
	additional_checks_after: Box<dyn Fn()>,
) where
	Runtime: frame_system::Config
		+ pallet_balances::Config
		+ pallet_session::Config
		+ pallet_collator_selection::Config
		+ cumulus_pallet_parachain_system::Config
		+ pallet_assets::Config<AssetsPalletInstance>,
	AccountIdOf<Runtime>: Into<[u8; 32]>,
	ValidatorIdOf<Runtime>: From<AccountIdOf<Runtime>>,
	BalanceOf<Runtime>: From<Balance>,
	XcmConfig: xcm_executor::Config,
	<Runtime as pallet_assets::Config<AssetsPalletInstance>>::AssetId:
		From<AssetIdForTrustBackedAssets> + Into<AssetIdForTrustBackedAssets>,
	<Runtime as pallet_assets::Config<AssetsPalletInstance>>::AssetIdParameter:
		From<AssetIdForTrustBackedAssets> + Into<AssetIdForTrustBackedAssets>,
	<Runtime as pallet_assets::Config<AssetsPalletInstance>>::Balance: From<Balance> + Into<u128>,
	<Runtime as frame_system::Config>::AccountId:
		Into<<<Runtime as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
	<<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source:
		From<<Runtime as frame_system::Config>::AccountId>,
	AssetsPalletInstance: 'static,
	TrustBackedAssetsPalletLocation: Get<MultiLocation>,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys.collators())
		.with_session_keys(collator_session_keys.session_keys())
		.with_balances(vec![
			(asset_owner.clone(), existential_deposit + asset_deposit),
			(alice_account.clone(), existential_deposit),
			(bob_account.clone(), existential_deposit),
		])
		.with_tracing()
		.build()
		.execute_with(|| {
			// create  some asset class
			let asset_minimum_asset_balance = 3333333_u128;
			let local_asset_id = 1;
			let local_asset_id_as_multilocation = AssetIdForTrustBackedAssetsConvert::<
				TrustBackedAssetsPalletLocation,
			>::reverse_ref(local_asset_id)
			.unwrap();
			assert_ok!(<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::create(
				RuntimeHelper::<Runtime>::origin_of(asset_owner.clone()),
				local_asset_id.into(),
				asset_owner.clone().into(),
				asset_minimum_asset_balance.into()
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::mint(
				RuntimeHelper::<Runtime>::origin_of(asset_owner.clone()),
				local_asset_id.into(),
				alice_account.clone().into(),
				(6 * asset_minimum_asset_balance).into()
			));

			// check Assets before
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&alice_account
				),
				(6 * asset_minimum_asset_balance).into()
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&bob_account
				),
				0.into()
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&charlie_account
				),
				0.into()
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&asset_owner
				),
				0.into()
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&alice_account),
				existential_deposit
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&bob_account),
				existential_deposit
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&charlie_account),
				0.into()
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&asset_owner),
				existential_deposit
			);
			additional_checks_before();

			// transfer_asset (deposit/withdraw) ALICE -> CHARLIE (not ok - Charlie does not have ExistentialDeposit)
			assert_noop!(
				RuntimeHelper::<XcmConfig>::do_transfer(
					MultiLocation {
						parents: 0,
						interior: X1(AccountId32 {
							network: None,
							id: alice_account.clone().into()
						}),
					},
					MultiLocation {
						parents: 0,
						interior: X1(AccountId32 {
							network: None,
							id: charlie_account.clone().into()
						}),
					},
					(local_asset_id_as_multilocation, 1 * asset_minimum_asset_balance),
				),
				XcmError::FailedToTransactAsset(Into::<&str>::into(
					sp_runtime::TokenError::CannotCreate
				))
			);

			// transfer_asset (deposit/withdraw) ALICE -> BOB (ok - has ExistentialDeposit)
			assert!(matches!(
				RuntimeHelper::<XcmConfig>::do_transfer(
					MultiLocation {
						parents: 0,
						interior: X1(AccountId32 {
							network: None,
							id: alice_account.clone().into()
						}),
					},
					MultiLocation {
						parents: 0,
						interior: X1(AccountId32 { network: None, id: bob_account.clone().into() }),
					},
					(local_asset_id_as_multilocation, 1 * asset_minimum_asset_balance),
				),
				Ok(_)
			));

			// check Assets after
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&alice_account
				),
				(5 * asset_minimum_asset_balance).into()
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&bob_account
				),
				(1 * asset_minimum_asset_balance).into()
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&charlie_account
				),
				0.into()
			);
			assert_eq!(
				<pallet_assets::Pallet<Runtime, AssetsPalletInstance>>::balance(
					local_asset_id.into(),
					&asset_owner
				),
				0.into()
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&alice_account),
				existential_deposit
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&bob_account),
				existential_deposit
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&charlie_account),
				0.into()
			);
			assert_eq!(
				<pallet_balances::Pallet<Runtime>>::free_balance(&asset_owner),
				existential_deposit
			);
			additional_checks_after();
		})
}

#[macro_export]
macro_rules! include_asset_transactor_transfer_with_trust_backed_assets_works(
	(
		$runtime:path,
		$xcm_config:path,
		$assets_pallet_instance:path,
		$trust_backed_assets_pallet_location:path,
		$collator_session_key:expr,
		$existential_deposit:expr,
		$asset_deposit:expr,
		$additional_checks_before:expr,
		$additional_checks_after:expr
	) => {
		#[test]
		fn asset_transactor_transfer_with_trust_backed_assets_works() {
			const SOME_ASSET_OWNER: [u8; 32] = [5u8; 32];
			let asset_owner = parachains_common::AccountId::from(SOME_ASSET_OWNER);
			const ALICE: [u8; 32] = [1u8; 32];
			let alice_account = parachains_common::AccountId::from(ALICE);
			const BOB: [u8; 32] = [2u8; 32];
			let bob_account = parachains_common::AccountId::from(BOB);
			const CHARLIE: [u8; 32] = [3u8; 32];
			let charlie_account = parachains_common::AccountId::from(CHARLIE);

			asset_test_utils::test_cases::asset_transactor_transfer_with_trust_backed_assets_works::<
				$runtime,
				$xcm_config,
				$assets_pallet_instance,
				$trust_backed_assets_pallet_location
			>(
				$collator_session_key,
				$existential_deposit,
				$asset_deposit,
				asset_owner,
				alice_account,
				bob_account,
				charlie_account,
				$additional_checks_before,
				$additional_checks_after
			)
		}
	}
);
