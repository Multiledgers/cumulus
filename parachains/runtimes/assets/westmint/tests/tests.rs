use asset_test_utils::{ExtBuilder, RuntimeHelper};
use codec::Encode;
use cumulus_primitives_utility::ChargeWeightInFungibles;
use frame_support::{
	assert_noop, assert_ok, sp_io,
	traits::{fungibles::InspectEnumerable, PalletInfo},
	weights::{Weight, WeightToFee as WeightToFeeT},
};
use parachains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId};
use std::convert::Into;
pub use westmint_runtime::{
	constants::fee::WeightToFee, xcm_config::XcmConfig, AssetDeposit, Assets, Balances,
	ExistentialDeposit, ForeignAssets, ForeignAssetsInstance, Runtime, SessionKeys, System,
};
use westmint_runtime::{
	xcm_config::{
		AssetFeeAsExistentialDepositMultiplierFeeCharger, ForeignCreatorsSovereignAccountOf,
		TrustBackedAssetsPalletLocation,
	},
	RuntimeCall, RuntimeEvent,
};
use xcm::latest::prelude::*;
use xcm_builder::AsPrefixedGeneralIndex;
use xcm_executor::{
	traits::{Convert, JustTry, WeightTrader},
	XcmExecutor,
};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const CHARLIE: [u8; 32] = [3u8; 32];
pub const SOME_ASSET_OWNER: [u8; 32] = [4u8; 32];

#[test]
fn test_asset_xcm_trader() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			let minimum_asset_balance = 3333333_u128;
			let local_asset_id = 1;
			assert_ok!(Assets::force_create(
				RuntimeHelper::<Runtime>::root_origin(),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				true,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::<Runtime>::origin_of(AccountId::from(ALICE)),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			// get asset id as multilocation
			let asset_multilocation = MultiLocation::new(
				0,
				X2(
					PalletInstance(
						<Runtime as frame_system::Config>::PalletInfo::index::<Assets>().unwrap()
							as u8,
					),
					GeneralIndex(local_asset_id.into()),
				),
			);

			// Set Alice as block author, who will receive fees
			RuntimeHelper::<Runtime>::run_to_block(2, Some(AccountId::from(ALICE)));

			// We are going to buy 4e9 weight
			let bought = Weight::from_ref_time(4_000_000_000u64);

			// Lets calculate amount needed
			let asset_amount_needed =
				AssetFeeAsExistentialDepositMultiplierFeeCharger::charge_weight_in_fungibles(
					local_asset_id,
					bought,
				)
				.expect("failed to compute");

			// Lets pay with: asset_amount_needed + asset_amount_extra
			let asset_amount_extra = 100_u128;
			let asset: MultiAsset =
				(asset_multilocation.clone(), asset_amount_needed + asset_amount_extra).into();

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();

			// Lets buy_weight and make sure buy_weight does not return an error
			match trader.buy_weight(bought, asset.into()) {
				Ok(unused_assets) => {
					// Check whether a correct amount of unused assets is returned
					assert_ok!(unused_assets
						.ensure_contains(&(asset_multilocation, asset_amount_extra).into()));
				},
				Err(e) => assert!(false, "Expected Ok(_). Got {:#?}", e),
			}

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has received the amount
			assert_eq!(
				Assets::balance(1, AccountId::from(ALICE)),
				minimum_asset_balance + asset_amount_needed
			);

			// We also need to ensure the total supply increased
			assert_eq!(Assets::total_supply(1), minimum_asset_balance + asset_amount_needed);
		});
}

#[test]
fn test_asset_xcm_trader_with_refund() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::<Runtime>::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::<Runtime>::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();

			// Set Alice as block author, who will receive fees
			RuntimeHelper::<Runtime>::run_to_block(2, Some(AccountId::from(ALICE)));

			// We are going to buy 4e9 weight
			let bought = Weight::from_ref_time(4_000_000_000u64);
			let asset_multilocation = MultiLocation::new(
				0,
				X2(
					PalletInstance(
						<Runtime as frame_system::Config>::PalletInfo::index::<Assets>().unwrap()
							as u8,
					),
					GeneralIndex(1),
				),
			);

			// lets calculate amount needed
			let amount_bought = WeightToFee::weight_to_fee(&bought);

			let asset: MultiAsset = (asset_multilocation.clone(), amount_bought).into();

			// Make sure buy_weight does not return an error
			assert_ok!(trader.buy_weight(bought, asset.clone().into()));

			// Make sure again buy_weight does return an error
			assert_noop!(trader.buy_weight(bought, asset.into()), XcmError::NotWithdrawable);

			// We actually use half of the weight
			let weight_used = bought / 2;

			// Make sure refurnd works.
			let amount_refunded = WeightToFee::weight_to_fee(&(bought - weight_used));

			assert_eq!(
				trader.refund_weight(bought - weight_used),
				Some((asset_multilocation, amount_refunded).into())
			);

			// Drop trader
			drop(trader);

			// We only should have paid for half of the bought weight
			let fees_paid = WeightToFee::weight_to_fee(&weight_used);

			assert_eq!(
				Assets::balance(1, AccountId::from(ALICE)),
				ExistentialDeposit::get() + fees_paid
			);

			// We also need to ensure the total supply increased
			assert_eq!(Assets::total_supply(1), ExistentialDeposit::get() + fees_paid);
		});
}

#[test]
fn test_asset_xcm_trader_refund_not_possible_since_amount_less_than_ed() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::<Runtime>::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();

			// Set Alice as block author, who will receive fees
			RuntimeHelper::<Runtime>::run_to_block(2, Some(AccountId::from(ALICE)));

			// We are going to buy 5e9 weight
			let bought = Weight::from_ref_time(500_000_000u64);

			let asset_multilocation = MultiLocation::new(
				0,
				X2(
					PalletInstance(
						<Runtime as frame_system::Config>::PalletInfo::index::<Assets>().unwrap()
							as u8,
					),
					GeneralIndex(1),
				),
			);

			let amount_bought = WeightToFee::weight_to_fee(&bought);

			assert!(
				amount_bought < ExistentialDeposit::get(),
				"we are testing what happens when the amount does not exceed ED"
			);

			let asset: MultiAsset = (asset_multilocation.clone(), amount_bought).into();

			// Buy weight should return an error
			assert_noop!(trader.buy_weight(bought, asset.into()), XcmError::TooExpensive);

			// not credited since the ED is higher than this value
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), 0);

			// We also need to ensure the total supply did not increase
			assert_eq!(Assets::total_supply(1), 0);
		});
}

#[test]
fn test_that_buying_ed_refund_does_not_refund() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// We need root origin to create a sufficient asset
			// We set existential deposit to be identical to the one for Balances first
			assert_ok!(Assets::force_create(
				RuntimeHelper::<Runtime>::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				true,
				ExistentialDeposit::get()
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();

			// Set Alice as block author, who will receive fees
			RuntimeHelper::<Runtime>::run_to_block(2, Some(AccountId::from(ALICE)));

			let bought = Weight::from_ref_time(500_000_000u64);

			let asset_multilocation = MultiLocation::new(
				0,
				X2(
					PalletInstance(
						<Runtime as frame_system::Config>::PalletInfo::index::<Assets>().unwrap()
							as u8,
					),
					GeneralIndex(1),
				),
			);

			let amount_bought = WeightToFee::weight_to_fee(&bought);

			assert!(
				amount_bought < ExistentialDeposit::get(),
				"we are testing what happens when the amount does not exceed ED"
			);

			// We know we will have to buy at least ED, so lets make sure first it will
			// fail with a payment of less than ED
			let asset: MultiAsset = (asset_multilocation.clone(), amount_bought).into();
			assert_noop!(trader.buy_weight(bought, asset.into()), XcmError::TooExpensive);

			// Now lets buy ED at least
			let asset: MultiAsset = (asset_multilocation.clone(), ExistentialDeposit::get()).into();

			// Buy weight should work
			assert_ok!(trader.buy_weight(bought, asset.into()));

			// Should return None. We have a specific check making sure we dont go below ED for
			// drop payment
			assert_eq!(trader.refund_weight(bought), None);

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has received the amount
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), ExistentialDeposit::get());

			// We also need to ensure the total supply increased
			assert_eq!(Assets::total_supply(1), ExistentialDeposit::get());
		});
}

#[test]
fn test_asset_xcm_trader_not_possible_for_non_sufficient_assets() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// Create a non-sufficient asset with specific existential deposit
			let minimum_asset_balance = 1_000_000_u128;
			assert_ok!(Assets::force_create(
				RuntimeHelper::<Runtime>::root_origin(),
				1.into(),
				AccountId::from(ALICE).into(),
				false,
				minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::<Runtime>::origin_of(AccountId::from(ALICE)),
				1.into(),
				AccountId::from(ALICE).into(),
				minimum_asset_balance
			));

			let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();

			// Set Alice as block author, who will receive fees
			RuntimeHelper::<Runtime>::run_to_block(2, Some(AccountId::from(ALICE)));

			// We are going to buy 4e9 weight
			let bought = Weight::from_ref_time(4_000_000_000u64);

			// lets calculate amount needed
			let asset_amount_needed = WeightToFee::weight_to_fee(&bought);

			let asset_multilocation = MultiLocation::new(
				0,
				X2(
					PalletInstance(
						<Runtime as frame_system::Config>::PalletInfo::index::<Assets>().unwrap()
							as u8,
					),
					GeneralIndex(1),
				),
			);

			let asset: MultiAsset = (asset_multilocation, asset_amount_needed).into();

			// Make sure again buy_weight does return an error
			assert_noop!(trader.buy_weight(bought, asset.into()), XcmError::TooExpensive);

			// Drop trader
			drop(trader);

			// Make sure author(Alice) has NOT received the amount
			assert_eq!(Assets::balance(1, AccountId::from(ALICE)), minimum_asset_balance);

			// We also need to ensure the total supply NOT increased
			assert_eq!(Assets::total_supply(1), minimum_asset_balance);
		});
}

#[test]
fn test_asset_transactor_transfer_with_local_consensus_currency_works() {
	let unit = ExistentialDeposit::get();

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.with_balances(vec![(AccountId::from(ALICE), 10 * unit)])
		.with_tracing()
		.build()
		.execute_with(|| {
			// check Balances before
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 10 * unit);
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), 0 * unit);
			assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
			assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());

			// transfer_asset (deposit/withdraw) ALICE -> BOB
			let _ = RuntimeHelper::<XcmConfig>::do_transfer(
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: AccountId::from(ALICE).into() }),
				},
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: AccountId::from(BOB).into() }),
				},
				// local_consensus_currency_asset, e.g.: relaychain token (KSM, DOT, ...)
				(MultiLocation { parents: 1, interior: Here }, 1 * unit),
			)
			.expect("no error");

			// check Balances after
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), 9 * unit);
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), 1 * unit);
			assert!(Assets::asset_ids().collect::<Vec<_>>().is_empty());
			assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
		})
}

#[test]
fn test_asset_transactor_transfer_with_trust_backed_assets_works() {
	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.with_balances(vec![
			(AccountId::from(SOME_ASSET_OWNER), ExistentialDeposit::get() + AssetDeposit::get()),
			(AccountId::from(ALICE), ExistentialDeposit::get()),
			(AccountId::from(BOB), ExistentialDeposit::get())
		])
		.with_tracing()
		.build()
		.execute_with(|| {
			// create  some asset class
			let asset_minimum_asset_balance = 3333333_u128;
			let local_asset_id = 1;
			let local_asset_id_as_multilocation = {
				type AssetIdConverter = AsPrefixedGeneralIndex<
					TrustBackedAssetsPalletLocation,
					AssetIdForTrustBackedAssets,
					JustTry,
				>;
				AssetIdConverter::reverse_ref(local_asset_id).unwrap()
			};
			assert_ok!(Assets::create(
				RuntimeHelper::<Runtime>::origin_of(AccountId::from(SOME_ASSET_OWNER)),
				local_asset_id.into(),
				AccountId::from(SOME_ASSET_OWNER).into(),
				asset_minimum_asset_balance
			));

			// We first mint enough asset for the account to exist for assets
			assert_ok!(Assets::mint(
				RuntimeHelper::<Runtime>::origin_of(AccountId::from(SOME_ASSET_OWNER)),
				local_asset_id.into(),
				AccountId::from(ALICE).into(),
				6 * asset_minimum_asset_balance
			));

			// check Assets before
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				6 * asset_minimum_asset_balance
			);
			assert_eq!(Assets::balance(local_asset_id, AccountId::from(BOB)), 0);
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(CHARLIE)),
				0
			);
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(SOME_ASSET_OWNER)),
				0
			);
			assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
			assert_eq!(Balances::free_balance(AccountId::from(SOME_ASSET_OWNER)), ExistentialDeposit::get());
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), ExistentialDeposit::get());
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), ExistentialDeposit::get());
			assert_eq!(Balances::free_balance(AccountId::from(CHARLIE)), 0);

			// transfer_asset (deposit/withdraw) ALICE -> CHARLIE (not ok - Charlie does not have ExistentialDeposit)
			assert!(matches!(
				RuntimeHelper::<XcmConfig>::do_transfer(
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: AccountId::from(ALICE).into() }),
				},
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: AccountId::from(CHARLIE).into() }),
				},
				(local_asset_id_as_multilocation, 1 * asset_minimum_asset_balance),
				),
				Err(XcmError::FailedToTransactAsset(reason)) if reason == Into::<&str>::into(sp_runtime::TokenError::CannotCreate)
			));

			// transfer_asset (deposit/withdraw) ALICE -> BOB (ok - has ExistentialDeposit)
			assert!(matches!(
				RuntimeHelper::<XcmConfig>::do_transfer(
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: AccountId::from(ALICE).into() }),
				},
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: AccountId::from(BOB).into() }),
				},
				(local_asset_id_as_multilocation, 1 * asset_minimum_asset_balance),
				),
				Ok(_)
			));

			// check Assets after
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(ALICE)),
				5 * asset_minimum_asset_balance
			);
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(BOB)),
				1 * asset_minimum_asset_balance
			);
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(CHARLIE)),
				0
			);
			assert_eq!(
				Assets::balance(local_asset_id, AccountId::from(SOME_ASSET_OWNER)),
				0
			);
			assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
			assert_eq!(Balances::free_balance(AccountId::from(SOME_ASSET_OWNER)), ExistentialDeposit::get());
			assert_eq!(Balances::free_balance(AccountId::from(ALICE)), ExistentialDeposit::get());
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), ExistentialDeposit::get());
			assert_eq!(Balances::free_balance(AccountId::from(CHARLIE)), 0);
		})
}

#[test]
fn test_create_foreign_assets_for_different_consensus_relaychain_token_works() {
	let foreign_creator = MultiLocation { parents: 2, interior: X1(GlobalConsensus(Kusama)) };
	let foreign_creator_as_account_id =
		ForeignCreatorsSovereignAccountOf::convert(foreign_creator).expect("");

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.with_balances(vec![(
			foreign_creator_as_account_id.clone(),
			ExistentialDeposit::get() + AssetDeposit::get(),
		)])
		.with_tracing()
		.build()
		.execute_with(|| {
			assert!(ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());

			// execute XCM with Transact to create foreign asset
			let foreign_asset_id_multilocation =
				MultiLocation { parents: 2, interior: X1(GlobalConsensus(Kusama)) };

			let foreign_asset_create: RuntimeCall =
				RuntimeCall::Utility(pallet_utility::Call::<Runtime>::batch {
					calls: vec![
						RuntimeCall::ForeignAssets(pallet_assets::Call::<
							Runtime,
							ForeignAssetsInstance,
						>::create {
							id: foreign_asset_id_multilocation,
							// TODO:check-parameter - how to setup admin account?
							admin: foreign_creator_as_account_id.clone().into(),
							min_balance: 1,
						}),
						// TODO:check-parameter - cannot call in one batch, because set_metadata uses just `ensure_signed()`
						// RuntimeCall::ForeignAssets(pallet_assets::Call::<
						// 	Runtime,
						// 	ForeignAssetsInstance,
						// >::set_metadata {
						// 	id: foreign_asset_id_multilocation,
						// 	name: Default::default(),
						// 	symbol: Default::default(),
						// 	decimals: 12,
						// }),
					],
				});

			// lets simulate this was triggered by relay chain from different global consensus
			let xcm = Xcm(vec![
				UniversalOrigin(GlobalConsensus(Kusama)),
				DescendOrigin(Here),
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					require_weight_at_most: Weight::from_ref_time(80_000_000_000),
					call: foreign_asset_create.encode().into(),
				},
			]);

			// messages with different consensus should go through the local bridge-hub
			let local_bridge_hub_origin =
				MultiLocation { parents: 1, interior: X1(Parachain(1014)) };
			let hash = xcm.using_encoded(sp_io::hashing::blake2_256);
			let weight_limit = Weight::from_ref_time(100_000_000_000);

			// execute xcm as XcmpQueue would do
			let outcome = XcmExecutor::<XcmConfig>::execute_xcm(
				local_bridge_hub_origin,
				xcm,
				hash,
				weight_limit,
			);
			assert_eq!(outcome.ensure_complete(), Ok(()));

			// check events
			let mut events = System::events().into_iter().map(|e| e.event);
			assert!(events.any(|e| matches!(
				e,
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { .. })
			)));
			// TODO:check-parameter - cannot call in one batch, because set_metadata uses just `ensure_signed()`
			// assert!(
			// 	events
			// 		.any(|e| matches!(e, RuntimeEvent::ForeignAssets(pallet_assets::Event::MetadataSet { .. })))
			// );
			assert!(
				events.any(|e| e.eq(&RuntimeEvent::Utility(pallet_utility::Event::BatchCompleted)))
			);

			// check assets after
			assert!(!ForeignAssets::asset_ids().collect::<Vec<_>>().is_empty());
			use frame_support::traits::tokens::fungibles::roles::Inspect;
			assert_eq!(
				ForeignAssets::owner(foreign_asset_id_multilocation),
				Some(foreign_creator_as_account_id.clone())
			);
			assert_eq!(
				ForeignAssets::admin(foreign_asset_id_multilocation),
				Some(foreign_creator_as_account_id)
			);
		})
}

// TODO: fn test_create_foreign_assets_for_different_consensus_parachain_assets_works
// TODO: fn test_asset_transactor_transfer_with_foreign_assets_works
// TODO: test that simulates ETH scenario for creating asset wETH on Statemine/t
