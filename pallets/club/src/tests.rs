use crate::{
	mock::{new_test_ext, Balances, Club, RuntimeEvent, RuntimeOrigin, System, Test},
	pallet::{ClubMembership, Clubs, ExpiredMemberships, MembershipRequest, NextClubId},
	ClubDetails, ClubMemberFutureExpirations, Config, Error, Event, ExpirationDetails,
	ExpirationsPerBlock, MembershipDetails, MembershipRequestDetails,
};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use sp_runtime::TokenError;

#[test]
fn club_creation_test() {
	new_test_ext(1, 1, vec![(2, 100)]).execute_with(|| {
		// Only root account can create club
		assert_noop!(
			Club::create_club(RuntimeOrigin::signed(2), 2, 100),
			Error::<Test>::UserIsNotRoot
		);
		// Root account need sufficient balance
		assert_noop!(
			Club::create_club(RuntimeOrigin::signed(1), 2, 100),
			TokenError::FundsUnavailable
		);
	});

	new_test_ext(1, 1, vec![(1, 100), (2, 100)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// Balance of root account should now be less by club creation fee
		assert_eq!(Balances::free_balance(1), 100 - 1);

		assert_eq!(Clubs::<Test>::get(1), Some(ClubDetails { owner: 2, expense_per_year: 100 }));
		assert_eq!(NextClubId::<Test>::get(), 2);

		System::assert_last_event(RuntimeEvent::Club(Event::ClubCreated {
			club_id: 1,
			club_owner: 2,
			annual_expense: 100,
		}));
	});
}

#[test]
fn transfer_club_ownership_test() {
	new_test_ext(1, 1, vec![(1, 100), (2, 100), (3, 1)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// Only owner can transfer the ownership to another owner
		assert_noop!(
			Club::transfer_club_ownership(RuntimeOrigin::signed(1), 1, 3),
			Error::<Test>::NotClubOwner
		);
		assert_noop!(
			Club::transfer_club_ownership(RuntimeOrigin::signed(3), 1, 1),
			Error::<Test>::NotClubOwner
		);

		// Club in question should exist
		assert_noop!(
			Club::transfer_club_ownership(RuntimeOrigin::signed(2), 2, 3),
			Error::<Test>::ClubNotFound
		);

		assert_ok!(Club::transfer_club_ownership(RuntimeOrigin::signed(2), 1, 3));

		// Storage check
		assert_eq!(Clubs::<Test>::get(1), Some(ClubDetails { owner: 3, expense_per_year: 100 }));

		// Event check
		System::assert_last_event(RuntimeEvent::Club(Event::ClubOwnerChanged {
			club_id: 1,
			old_owner: 2,
			new_owner: 3,
		}));
	});
}

#[test]
fn change_club_expense_test() {
	new_test_ext(1, 1, vec![(1, 100), (2, 100), (3, 1)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// Only owner can change expense
		assert_noop!(
			Club::change_club_expense(RuntimeOrigin::signed(1), 1, 200),
			Error::<Test>::NotClubOwner
		);

		// Club in question should exist
		assert_noop!(
			Club::change_club_expense(RuntimeOrigin::signed(2), 2, 200),
			Error::<Test>::ClubNotFound
		);

		assert_ok!(Club::change_club_expense(RuntimeOrigin::signed(2), 1, 200));

		// Storage check
		assert_eq!(Clubs::<Test>::get(1), Some(ClubDetails { owner: 2, expense_per_year: 200 }));

		// Event check
		System::assert_last_event(RuntimeEvent::Club(Event::AnnualExpenseSet {
			club_id: 1,
			old_annual_expense: 100,
			new_annual_expense: 200,
		}));
	});
}

#[test]
fn request_membership_test() {
	new_test_ext(1, 1, vec![(1, 100), (2, 100), (3, 200)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// Club id must exists
		assert_noop!(
			Club::request_membership(RuntimeOrigin::signed(3), 2, 100),
			Error::<Test>::ClubNotFound
		);

		// If membership is already requested it should fail
		MembershipRequest::<Test>::set(
			3,
			1,
			Some(MembershipRequestDetails { amount_paid: 100, time_in_year: 1, is_renewal: false }),
		);
		assert_noop!(
			Club::request_membership(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::MembershipAlreadyRequested
		);
		MembershipRequest::<Test>::remove(3, 1);

		// If already member, it cannot be member again
		ClubMembership::<Test>::set(3, 1, Some(MembershipDetails { is_renewal: false }));
		assert_noop!(
			Club::request_membership(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::AlreadyMember
		);
		ClubMembership::<Test>::remove(3, 1);

		// If there is expired membership, it cannot request new membership
		ExpiredMemberships::<Test>::set(
			3,
			1,
			Some(ExpirationDetails {
				previous_membership_details: MembershipDetails { is_renewal: false },
			}),
		);
		assert_noop!(
			Club::request_membership(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::ExpiredMember
		);
		ExpiredMemberships::<Test>::remove(3, 1);

		// Membership time cannot be more than max number of years (value: 100)
		assert_noop!(
			Club::request_membership(RuntimeOrigin::signed(3), 1, 150),
			Error::<Test>::MembershipTimeExceeded
		);

		assert_ok!(Club::request_membership(RuntimeOrigin::signed(3), 1, 1));

		// Storage check
		assert_eq!(
			MembershipRequest::<Test>::get(3, 1),
			Some(MembershipRequestDetails { amount_paid: 100, time_in_year: 1, is_renewal: false })
		);

		// Balance check
		assert_eq!(Balances::free_balance(3), 200 - 100);

		// Event check
		System::assert_last_event(RuntimeEvent::Club(Event::MembershipRequested {
			club_id: 1,
			requester: 3,
			expense_to_be_charged: 100,
			time_in_year: 1,
			is_renewal: false,
		}));
	});
}

#[test]
fn request_membership_renewal_test() {
	new_test_ext(1, 1, vec![(1, 100), (2, 100), (3, 200)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// Club id must exists
		assert_noop!(
			Club::request_membership_renewal(RuntimeOrigin::signed(3), 2, 100),
			Error::<Test>::ClubNotFound
		);

		// If membership is already requested it should fail
		MembershipRequest::<Test>::set(
			3,
			1,
			Some(MembershipRequestDetails { amount_paid: 100, time_in_year: 1, is_renewal: true }),
		);
		assert_noop!(
			Club::request_membership_renewal(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::MembershipAlreadyRequested
		);
		MembershipRequest::<Test>::remove(3, 1);

		// If already member, it cannot be member again
		ClubMembership::<Test>::set(3, 1, Some(MembershipDetails { is_renewal: true }));
		assert_noop!(
			Club::request_membership_renewal(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::AlreadyMember
		);
		ClubMembership::<Test>::remove(3, 1);

		// If there is not expired membership, it cannot request new membership
		assert_noop!(
			Club::request_membership_renewal(RuntimeOrigin::signed(3), 1, 100),
			Error::<Test>::NoMembershipExpirationFound
		);
		ExpiredMemberships::<Test>::remove(3, 1);

		// Membership time cannot be more than max number of years (value: 100)
		ExpiredMemberships::<Test>::set(
			3,
			1,
			Some(ExpirationDetails {
				previous_membership_details: MembershipDetails { is_renewal: false },
			}),
		);
		assert_noop!(
			Club::request_membership_renewal(RuntimeOrigin::signed(3), 1, 150),
			Error::<Test>::MembershipTimeExceeded
		);
		ExpiredMemberships::<Test>::remove(3, 1);

		ExpiredMemberships::<Test>::set(
			3,
			1,
			Some(ExpirationDetails {
				previous_membership_details: MembershipDetails { is_renewal: false },
			}),
		);
		assert_ok!(Club::request_membership_renewal(RuntimeOrigin::signed(3), 1, 1));

		// Storage check
		assert_eq!(
			MembershipRequest::<Test>::get(3, 1),
			Some(MembershipRequestDetails { amount_paid: 100, time_in_year: 1, is_renewal: true })
		);

		// Balance check
		assert_eq!(Balances::free_balance(3), 200 - 100);

		// Event check
		System::assert_last_event(RuntimeEvent::Club(Event::MembershipRequested {
			club_id: 1,
			requester: 3,
			expense_to_be_charged: 100,
			time_in_year: 1,
			is_renewal: true,
		}));
	});
}

#[test]
fn add_member_test() {
	new_test_ext(1, 1, vec![(1, 100), (2, 100), (3, 200), (4, 200)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// AccountId: 3 requested membership
		assert_ok!(Club::request_membership(RuntimeOrigin::signed(3), 1, 1));

		// AccountId: 4 requested membership
		assert_ok!(Club::request_membership(RuntimeOrigin::signed(4), 1, 1));

		// Club id must be valid
		assert_noop!(Club::add_member(RuntimeOrigin::signed(2), 2, 3), Error::<Test>::ClubNotFound);

		// Only club owner can add member
		assert_noop!(Club::add_member(RuntimeOrigin::signed(3), 1, 3), Error::<Test>::NotClubOwner);

		// Setting block number
		System::set_block_number(100);

		// Addition of AccountId: 3 as member
		assert_ok!(Club::add_member(RuntimeOrigin::signed(2), 1, 3));

		// Storage checks
		assert_eq!(
			ExpirationsPerBlock::<Test>::get(100 + <Test as Config>::BlocksPerYear::get()),
			Some(1)
		);
		assert_eq!(
			ClubMemberFutureExpirations::<Test>::get((
				100 + <Test as Config>::BlocksPerYear::get(),
				1
			)),
			Some((3, 1))
		);
		assert_eq!(MembershipRequest::<Test>::get(3, 1), None);
		assert_eq!(
			ClubMembership::<Test>::get(3, 1),
			Some(MembershipDetails { is_renewal: false })
		);

		// Event check
		System::assert_last_event(RuntimeEvent::Club(Event::MemberAdded {
			club_id: 1,
			member: 3,
			membership_expiry_block: 100 + <Test as Config>::BlocksPerYear::get(),
		}));

		// Adding another member at same block, should increment block expiration count
		assert_ok!(Club::add_member(RuntimeOrigin::signed(2), 1, 4));

		// Storage checks
		assert_eq!(
			ExpirationsPerBlock::<Test>::get(100 + <Test as Config>::BlocksPerYear::get()),
			Some(2)
		);
		assert_eq!(
			ClubMemberFutureExpirations::<Test>::get((
				100 + <Test as Config>::BlocksPerYear::get(),
				2
			)),
			Some((4, 1))
		);
		assert_eq!(MembershipRequest::<Test>::get(4, 1), None);
		assert_eq!(
			ClubMembership::<Test>::get(4, 1),
			Some(MembershipDetails { is_renewal: false })
		);

		// Event check
		System::assert_last_event(RuntimeEvent::Club(Event::MemberAdded {
			club_id: 1,
			member: 4,
			membership_expiry_block: 100 + <Test as Config>::BlocksPerYear::get(),
		}));
	});
}

#[test]
fn block_initialization_test() {
	new_test_ext(1, 1, vec![(1, 100), (2, 100), (3, 200), (4, 200), (5, 200)]).execute_with(|| {
		// Only root account can create club
		assert_ok!(Club::create_club(RuntimeOrigin::signed(1), 2, 100));

		// AccountId: 3 requested membership
		assert_ok!(Club::request_membership(RuntimeOrigin::signed(3), 1, 1));

		// AccountId: 4 requested membership
		assert_ok!(Club::request_membership(RuntimeOrigin::signed(4), 1, 1));

		// Setting block number
		System::set_block_number(100);

		// Addition of AccountId: 3 as member
		assert_ok!(Club::add_member(RuntimeOrigin::signed(2), 1, 3));

		// Addition of AccountId: 4 as member
		assert_ok!(Club::add_member(RuntimeOrigin::signed(2), 1, 4));

		// Before the designated block, no processing happens
		Club::on_initialize(101);
		assert_eq!(
			ExpirationsPerBlock::<Test>::get(100 + <Test as Config>::BlocksPerYear::get()),
			Some(2)
		);
		assert_eq!(
			ClubMemberFutureExpirations::<Test>::get((
				100 + <Test as Config>::BlocksPerYear::get(),
				1
			)),
			Some((3, 1))
		);
		assert_eq!(
			ClubMemberFutureExpirations::<Test>::get((
				100 + <Test as Config>::BlocksPerYear::get(),
				2
			)),
			Some((4, 1))
		);
		assert_eq!(
			ClubMembership::<Test>::get(3, 1),
			Some(MembershipDetails { is_renewal: false })
		);
		assert_eq!(
			ClubMembership::<Test>::get(4, 1),
			Some(MembershipDetails { is_renewal: false })
		);

		// At the designated block's initialization, changes should happen
		Club::on_initialize(100 + <Test as Config>::BlocksPerYear::get());

		// Storage changes
		assert_eq!(
			ExpirationsPerBlock::<Test>::get(100 + <Test as Config>::BlocksPerYear::get()),
			None
		);
		assert_eq!(
			ClubMemberFutureExpirations::<Test>::get((
				100 + <Test as Config>::BlocksPerYear::get(),
				1
			)),
			None
		);
		assert_eq!(
			ClubMemberFutureExpirations::<Test>::get((
				100 + <Test as Config>::BlocksPerYear::get(),
				2
			)),
			None
		);
		assert_eq!(ClubMembership::<Test>::get(3, 1), None);
		assert_eq!(
			ExpiredMemberships::<Test>::get(3, 1),
			Some(ExpirationDetails {
				previous_membership_details: MembershipDetails { is_renewal: false },
			})
		);
		assert_eq!(ClubMembership::<Test>::get(4, 1), None);
		assert_eq!(
			ExpiredMemberships::<Test>::get(4, 1),
			Some(ExpirationDetails {
				previous_membership_details: MembershipDetails { is_renewal: false },
			})
		);

		// Events check
		System::assert_has_event(RuntimeEvent::Club(Event::MembershipExpired {
			club_id: 1,
			member: 3,
		}));
		System::assert_last_event(RuntimeEvent::Club(Event::MembershipExpired {
			club_id: 1,
			member: 4,
		}));
	});
}
