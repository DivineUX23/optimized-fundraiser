use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError
};
use crate::state::{Fundraiser};

#[inline(always)]
pub fn process_checker_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser,
        vault,
        maker_ata,
        _system_program,
        _token_program,
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    {
        let ata_state = unsafe { maker_ata.borrow_unchecked() };

        let ata_mint = unsafe { &*(ata_state.as_ptr() as *const [u8; 32]) };

        let ata_owner = unsafe { &*(ata_state.as_ptr().add(32) as *const [u8; 32]) };

        if ata_owner != maker.address().as_array() {
            return Err(ProgramError::IllegalOwner);
        }

        if ata_mint != mint_to_raise.address().as_array() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let bump = unsafe { *( data.as_ptr() ) };

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;

    let vault_amount = unsafe { (vault.borrow_unchecked().as_ptr().add(64) as *const u64).read_unaligned() };

    if vault_amount < fundraiser_data.amount_to_raise() {
        return Err(ProgramError::InvalidArgument);
    }


    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);


    pinocchio_token::instructions::Transfer::new(vault, maker_ata, fundraiser, vault_amount)
        .invoke_signed(&[signer])?;

    maker.set_lamports(maker.lamports() + fundraiser.lamports());
    fundraiser.set_lamports(0);

    let _ = fundraiser.close();

    Ok(())
}