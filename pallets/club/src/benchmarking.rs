//! Benchmarking setup for pallet-club
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Club;
use frame_benchmarking::v2::*;
use frame_support::traits::Hooks;
use frame_system::{Pallet as System, RawOrigin};
use sp_core::Get;
use sp_runtime::traits::CheckedAdd;

const SEED: u32 = 0;

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_support::{assert_ok, error as error_log};

	#[benchmark]
	fn create_club() {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let owner_account: T::AccountId = account("owner", 0, SEED);

		whitelist_account!(root_account);
		#[extrinsic_call]
		create_club(RawOrigin::Signed(root_account), owner_account.clone(), 100u32.into());

		assert_eq!(
			Clubs::<T>::get(1),
			Some(ClubDetails { owner: owner_account, expense_per_year: 100u32.into() })
		);
	}

	#[benchmark]
	fn transfer_club_ownership() {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let previous_owner: T::AccountId = account("previous_owner", 0, SEED);
		T::Currency::resolve_creating(&previous_owner, T::Currency::issue(1000u32.into()));
		assert_ok!(Club::<T>::create_club(
			RawOrigin::Signed(root_account).into(),
			previous_owner.clone(),
			100u32.into()
		));

		let new_owner: T::AccountId = account("new_owner", 0, SEED);

		whitelist_account!(previous_owner);
		#[extrinsic_call]
		transfer_club_ownership(RawOrigin::Signed(previous_owner), 1, new_owner.clone());

		assert_eq!(
			Clubs::<T>::get(1),
			Some(ClubDetails { owner: new_owner, expense_per_year: 100u32.into() })
		);
	}

	#[benchmark]
	fn change_club_expense() {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let owner: T::AccountId = account("owner", 0, SEED);
		T::Currency::resolve_creating(&owner, T::Currency::issue(1000u32.into()));
		assert_ok!(Club::<T>::create_club(
			RawOrigin::Signed(root_account).into(),
			owner.clone(),
			100u32.into()
		));

		whitelist_account!(owner);
		#[extrinsic_call]
		change_club_expense(RawOrigin::Signed(owner.clone()), 1, 200u32.into());

		assert_eq!(
			Clubs::<T>::get(1),
			Some(ClubDetails { owner, expense_per_year: 200u32.into() })
		);
	}

	#[benchmark]
	fn request_membership() {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let owner: T::AccountId = account("owner", 0, SEED);
		assert_ok!(Club::<T>::create_club(
			RawOrigin::Signed(root_account).into(),
			owner,
			100u32.into()
		));

		let requester: T::AccountId = account("requester", 0, SEED);
		T::Currency::resolve_creating(&requester, T::Currency::issue(1000u32.into()));

		whitelist_account!(requester);
		#[extrinsic_call]
		request_membership(RawOrigin::Signed(requester.clone()), 1, 5);

		assert_eq!(
			MembershipRequest::<T>::get(requester, 1),
			Some(MembershipRequestDetails {
				amount_paid: 500u32.into(),
				time_in_year: 5,
				is_renewal: false
			})
		);
	}

	#[benchmark]
	fn request_membership_renewal() {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let owner: T::AccountId = account("owner", 0, SEED);
		assert_ok!(Club::<T>::create_club(
			RawOrigin::Signed(root_account).into(),
			owner.clone(),
			100u32.into()
		));

		let expired_member: T::AccountId = account("expired_member", 0, SEED);
		T::Currency::resolve_creating(&expired_member, T::Currency::issue(1000u32.into()));
		ExpiredMemberships::<T>::set(
			expired_member.clone(),
			1,
			Some(ExpirationDetails {
				previous_membership_details: MembershipDetails { is_renewal: false },
			}),
		);

		whitelist_account!(expired_member);
		#[extrinsic_call]
		request_membership_renewal(RawOrigin::Signed(expired_member.clone()), 1, 5);

		assert_eq!(
			MembershipRequest::<T>::get(expired_member, 1),
			Some(MembershipRequestDetails {
				amount_paid: 500u32.into(),
				time_in_year: 5,
				is_renewal: true
			})
		);
	}

	#[benchmark]
	fn add_member() {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let owner: T::AccountId = account("owner", 0, SEED);
		T::Currency::resolve_creating(&owner, T::Currency::issue(1000u32.into()));
		assert_ok!(Club::<T>::create_club(
			RawOrigin::Signed(root_account).into(),
			owner.clone(),
			100u32.into()
		));

		let requester: T::AccountId = account("requester", 0, SEED);
		T::Currency::resolve_creating(&requester, T::Currency::issue(1000u32.into()));
		assert_ok!(Club::<T>::request_membership(
			RawOrigin::Signed(requester.clone()).into(),
			1,
			5
		));

		whitelist_account!(owner);
		#[extrinsic_call]
		add_member(RawOrigin::Signed(owner), 1, requester.clone());

		assert_eq!(
			ClubMembership::<T>::get(requester, 1),
			Some(MembershipDetails { is_renewal: false })
		);
	}

	#[benchmark]
	fn on_initialize(x: Linear<1, 10_000>) {
		let root_account: T::AccountId = account("root", 0, SEED);
		T::Currency::resolve_creating(&root_account, T::Currency::issue(1000u32.into()));
		RootAccount::<T>::put(root_account.clone());

		let owner: T::AccountId = account("owner", 0, SEED);
		T::Currency::resolve_creating(&owner, T::Currency::issue(1000u32.into()));
		assert_ok!(Club::<T>::create_club(
			RawOrigin::Signed(root_account).into(),
			owner.clone(),
			1u32.into()
		));

		let mut members: Vec<T::AccountId> = vec![];
		for i in 0..x {
			let member: T::AccountId = account("member", i, SEED);
			T::Currency::resolve_creating(&member, T::Currency::issue(1000u32.into()));
			members.push(member);
		}

		// Requesting memberships
		for member in &members {
			assert_ok!(Club::<T>::request_membership(
				RawOrigin::Signed(member.clone()).into(),
				1,
				1
			));
		}

		// Setting block number
		System::<T>::set_block_number(100u32.into());

		// Add members
		for member in &members {
			assert_ok!(Club::<T>::add_member(
				RawOrigin::Signed(owner.clone()).into(),
				1,
				member.clone()
			));
		}

		let expiry_block = T::BlocksPerYear::get()
			.checked_add(&T::BlockNumber::from(100u32))
			.expect("Expiry block calculation should not overflow");
		#[block]
		{
			Club::<T>::on_initialize(expiry_block);
		}

		for member in &members {
			assert_eq!(
				ExpiredMemberships::<T>::get(member.clone(), 1),
				Some(ExpirationDetails {
					previous_membership_details: MembershipDetails { is_renewal: false },
				})
			);
		}
	}

	impl_benchmark_test_suite!(
		Club,
		crate::mock::new_test_ext(whitelisted_caller(), 1u128, vec![]),
		crate::mock::Test
	);
}
