use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError
};
use pinocchio_system::instructions::CreateAccount;

use crate::{state::Fundraiser, MIN_AMOUNT_TO_RAISE, FUNDRAISER_RENT_EXEMPT};

#[inline(always)]
pub fn process_initialize_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser,
        vault,
        system_program,
        token_program,
        _associated_token_program
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };



    if fundraiser.owned_by(&crate::ID) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }


    let amount_to_raise = unsafe { (data.as_ptr() as *const u64).read_unaligned() };
    let time_started = unsafe { (data.as_ptr().add(8) as *const i64).read_unaligned() };
    let duration = unsafe { *(data.as_ptr().add(16)) };
    let bump = unsafe { *(data.as_ptr().add(17)) };

    let decimals = unsafe { *mint_to_raise.borrow_unchecked().as_ptr().add(44) };

    let scaled_min = MIN_AMOUNT_TO_RAISE.wrapping_mul(10u64.wrapping_pow(decimals as u32));
    if amount_to_raise < scaled_min {
        return Err(ProgramError::InvalidArgument);
    }



    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    CreateAccount {
        from: maker,
        to: fundraiser,
        lamports: FUNDRAISER_RENT_EXEMPT,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID
    }
    .invoke_signed(&[signer])?;

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;
    fundraiser_data.set_maker(maker.address());
    fundraiser_data.set_mint_to_raise(mint_to_raise.address());
    fundraiser_data.set_amount_to_raise(amount_to_raise);
    fundraiser_data.set_current_amount(0);
    fundraiser_data.set_time_started(time_started);
    fundraiser_data.duration = duration;
    fundraiser_data.bump = bump;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint_to_raise,
        token_program,
        system_program
    }
    .invoke()?;

    Ok(())
}