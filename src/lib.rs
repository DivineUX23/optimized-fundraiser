#![allow(unexpected_cfgs)]

use pinocchio::{
    AccountView, Address, ProgramResult, address::declare_id, entrypoint, error::ProgramError
};

mod instructions;
mod state;
mod constants;
mod test;

use instructions::*;
use constants::*;

entrypoint!(process_instruction);

declare_id!("96TFrsG998MvvrfuShRQmSemkzN555pnidGF4gquJsKr");

#[inline(always)]
pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8]
) -> ProgramResult {

    let discriminator = unsafe { *instruction_data.as_ptr() };
    let data = unsafe { instruction_data.get_unchecked(1..) };

    match discriminator {
        0 => process_initialize_instruction(accounts, data)?,
        1 => process_contribute_instruction(accounts, data)?,
        2 => process_checker_instruction(accounts, data)?,
        3 => process_refund_instruction(accounts, data)?,
        _ => Err(ProgramError::InvalidInstructionData)?
    }

    Ok(())
}