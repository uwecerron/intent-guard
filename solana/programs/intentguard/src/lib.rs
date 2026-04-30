use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    ed25519_program,
    hash::hashv,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    sysvar::instructions::{load_instruction_at_checked, ID as INSTRUCTIONS_ID},
};

declare_id!("Fg6PaFpoGXkYsidMpWxTWqkqeY5VQwB7HyRbF3HsWJk");

pub const MAX_SIGNERS: usize = 16;
pub const MAX_CANCELLERS: usize = 16;
pub const MAX_DATA_HASHES: usize = 32;
pub const DOMAIN: &[u8] = b"intentguard.solana.attestation.v1";

#[program]
pub mod intentguard {
    use super::*;

    pub fn init_vault(
        ctx: Context<InitVault>,
        threshold: u8,
        veto_threshold: u8,
        fresh_window_secs: i64,
        cooloff_secs: i64,
        execute_delay_secs: i64,
        min_proposal_lifetime_secs: i64,
        signers: Vec<Pubkey>,
    ) -> Result<()> {
        require!(!signers.is_empty(), IntentGuardError::BadConfig);
        require!(signers.len() <= MAX_SIGNERS, IntentGuardError::TooManySigners);
        require!(threshold > 0 && threshold as usize <= signers.len(), IntentGuardError::BadConfig);
        require!(veto_threshold > 0 && veto_threshold as usize <= signers.len(), IntentGuardError::BadConfig);
        require!(fresh_window_secs > 0, IntentGuardError::BadConfig);
        require!(
            min_proposal_lifetime_secs >= cooloff_secs + execute_delay_secs,
            IntentGuardError::BadConfig
        );

        let mut sorted = signers.clone();
        sorted.sort();
        sorted.dedup();
        require!(sorted.len() == signers.len(), IntentGuardError::DuplicateSigner);

        let vault = &mut ctx.accounts.vault;
        vault.authority = ctx.accounts.authority.key();
        vault.threshold = threshold;
        vault.veto_threshold = veto_threshold;
        vault.fresh_window_secs = fresh_window_secs;
        vault.cooloff_secs = cooloff_secs;
        vault.execute_delay_secs = execute_delay_secs;
        vault.min_proposal_lifetime_secs = min_proposal_lifetime_secs;
        vault.nonce = 0;
        vault.signers = signers;
        vault.bump = ctx.bumps.vault;

        emit!(VaultInitialized {
            vault: vault.key(),
            authority: vault.authority,
            threshold,
            veto_threshold,
        });

        Ok(())
    }

    pub fn queue_proposal(
        ctx: Context<QueueProposal>,
        target_program: Pubkey,
        adapter: Pubkey,
        intent_hash: [u8; 32],
        instruction_data_hash: [u8; 32],
        expires_at: i64,
        attestations: Vec<SignerAttestation>,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &ctx.accounts.vault;
        require!(
            expires_at >= clock.unix_timestamp + vault.min_proposal_lifetime_secs,
            IntentGuardError::ProposalExpired
        );
        require!(attestations.len() >= vault.threshold as usize, IntentGuardError::InsufficientSignatures);

        verify_attestations(
            vault,
            ctx.accounts.proposal.key(),
            target_program,
            adapter,
            intent_hash,
            instruction_data_hash,
            expires_at,
            &attestations,
            &ctx.accounts.instructions,
        )?;

        let proposal = &mut ctx.accounts.proposal;
        proposal.vault = vault.key();
        proposal.nonce = vault.nonce;
        proposal.target_program = target_program;
        proposal.adapter = adapter;
        proposal.intent_hash = intent_hash;
        proposal.instruction_data_hash = instruction_data_hash;
        proposal.queued_at = clock.unix_timestamp;
        proposal.expires_at = expires_at;
        proposal.cancel_count = 0;
        proposal.cancellers = Vec::new();
        proposal.state = ProposalState::Queued;
        proposal.bump = ctx.bumps.proposal;

        emit!(ProposalQueued {
            proposal: proposal.key(),
            vault: vault.key(),
            nonce: vault.nonce,
            target_program,
            intent_hash,
            execute_after: clock.unix_timestamp + vault.cooloff_secs + vault.execute_delay_secs,
            expires_at,
        });

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>, reason: String) -> Result<()> {
        let proposal = &mut ctx.accounts.proposal;
        let vault = &ctx.accounts.vault;
        require!(proposal.state == ProposalState::Queued, IntentGuardError::BadState);
        require!(vault.signers.contains(&ctx.accounts.signer.key()), IntentGuardError::NotSigner);
        require!(!proposal.cancellers.contains(&ctx.accounts.signer.key()), IntentGuardError::DuplicateSigner);
        require!(proposal.cancellers.len() < MAX_CANCELLERS, IntentGuardError::TooManySigners);

        proposal.cancellers.push(ctx.accounts.signer.key());
        proposal.cancel_count = proposal.cancel_count.saturating_add(1);

        emit!(ProposalCancelled {
            proposal: proposal.key(),
            signer: ctx.accounts.signer.key(),
            cancel_count: proposal.cancel_count,
            reason,
        });

        if proposal.cancel_count >= vault.veto_threshold {
            proposal.state = ProposalState::Cancelled;
        }

        Ok(())
    }

    /// Execute a queued proposal.
    ///
    /// The caller supplies the target instruction as remaining accounts:
    /// - remaining_accounts[0] must be the target program account.
    /// - remaining_accounts[1..] are forwarded to the target CPI.
    ///
    /// Production integrations should pair this with protocol-specific adapter validation.
    pub fn execute(ctx: Context<Execute>, instruction_data: Vec<u8>) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &mut ctx.accounts.vault;
        let proposal = &mut ctx.accounts.proposal;

        require!(proposal.state == ProposalState::Queued, IntentGuardError::BadState);
        require!(proposal.nonce == vault.nonce, IntentGuardError::BadNonce);
        require!(proposal.expires_at >= clock.unix_timestamp, IntentGuardError::ProposalExpired);
        require!(
            clock.unix_timestamp >= proposal.queued_at + vault.cooloff_secs + vault.execute_delay_secs,
            IntentGuardError::CooloffActive
        );
        require!(hashv(&[&instruction_data]).to_bytes() == proposal.instruction_data_hash, IntentGuardError::BadIntent);
        require!(!ctx.remaining_accounts.is_empty(), IntentGuardError::MissingTargetProgram);
        require!(ctx.remaining_accounts[0].key() == proposal.target_program, IntentGuardError::BadTarget);

        // Adapter validation is intentionally protocol-specific. A production adapter should decode
        // instruction_data, verify the canonical intent hash, and perform oracle/risk checks.
        require!(ctx.accounts.adapter.key() == proposal.adapter, IntentGuardError::BadAdapter);

        proposal.state = ProposalState::Executed;
        vault.nonce = vault.nonce.saturating_add(1);

        let target_program = ctx.remaining_accounts[0].key();
        let account_infos = ctx.remaining_accounts[1..].to_vec();
        let metas = account_infos
            .iter()
            .map(|account| {
                if account.is_writable {
                    AccountMeta::new(*account.key, account.is_signer)
                } else {
                    AccountMeta::new_readonly(*account.key, account.is_signer)
                }
            })
            .collect::<Vec<_>>();

        let ix = Instruction {
            program_id: target_program,
            accounts: metas,
            data: instruction_data,
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vault",
            vault.authority.as_ref(),
            &[vault.bump],
        ]];
        invoke_signed(&ix, &account_infos, signer_seeds)?;

        emit!(ProposalExecuted {
            proposal: proposal.key(),
            target_program,
        });

        Ok(())
    }
}

fn verify_attestations(
    vault: &Account<Vault>,
    proposal: Pubkey,
    target_program: Pubkey,
    adapter: Pubkey,
    intent_hash: [u8; 32],
    instruction_data_hash: [u8; 32],
    expires_at: i64,
    attestations: &[SignerAttestation],
    instructions: &UncheckedAccount,
) -> Result<()> {
    let clock = Clock::get()?;
    let mut seen = Vec::<Pubkey>::new();
    let mut oldest = i64::MAX;
    let mut newest = 0_i64;

    for attestation in attestations {
        require!(vault.signers.contains(&attestation.signer), IntentGuardError::BadSignature);
        require!(!seen.contains(&attestation.signer), IntentGuardError::DuplicateSigner);
        seen.push(attestation.signer);

        require!(attestation.expires_at >= clock.unix_timestamp, IntentGuardError::SignatureNotFresh);
        require!(attestation.expires_at <= expires_at, IntentGuardError::SignatureNotFresh);
        require!(attestation.signed_at <= clock.unix_timestamp, IntentGuardError::SignatureNotFresh);
        require!(
            clock.unix_timestamp - attestation.signed_at <= vault.fresh_window_secs,
            IntentGuardError::SignatureNotFresh
        );

        oldest = oldest.min(attestation.signed_at);
        newest = newest.max(attestation.signed_at);

        let message = attestation_message(
            vault.key(),
            proposal,
            vault.nonce,
            target_program,
            adapter,
            intent_hash,
            instruction_data_hash,
            attestation.signed_at,
            attestation.expires_at,
        );

        verify_ed25519_instruction(
            instructions,
            attestation.ed25519_instruction_index,
            attestation.signer,
            &message,
            &attestation.signature,
        )?;
    }

    require!(seen.len() >= vault.threshold as usize, IntentGuardError::InsufficientSignatures);
    require!(newest - oldest <= vault.fresh_window_secs, IntentGuardError::SignatureNotFresh);
    Ok(())
}

fn attestation_message(
    vault: Pubkey,
    proposal: Pubkey,
    nonce: u64,
    target_program: Pubkey,
    adapter: Pubkey,
    intent_hash: [u8; 32],
    instruction_data_hash: [u8; 32],
    signed_at: i64,
    expires_at: i64,
) -> Vec<u8> {
    let mut message = Vec::with_capacity(32 * 6 + 32);
    message.extend_from_slice(DOMAIN);
    message.extend_from_slice(vault.as_ref());
    message.extend_from_slice(proposal.as_ref());
    message.extend_from_slice(&nonce.to_le_bytes());
    message.extend_from_slice(target_program.as_ref());
    message.extend_from_slice(adapter.as_ref());
    message.extend_from_slice(&intent_hash);
    message.extend_from_slice(&instruction_data_hash);
    message.extend_from_slice(&signed_at.to_le_bytes());
    message.extend_from_slice(&expires_at.to_le_bytes());
    message
}

fn verify_ed25519_instruction(
    instructions: &UncheckedAccount,
    instruction_index: u8,
    signer: Pubkey,
    message: &[u8],
    signature: &[u8; 64],
) -> Result<()> {
    let ix = load_instruction_at_checked(instruction_index as usize, instructions)?;
    require!(ix.program_id == ed25519_program::ID, IntentGuardError::BadSignature);

    // Minimal structural check: the ed25519 instruction must carry signer, signature, and message.
    // Production code should use a strict parser for the ed25519 instruction header offsets.
    require!(ix.data.windows(32).any(|window| window == signer.as_ref()), IntentGuardError::BadSignature);
    require!(ix.data.windows(64).any(|window| window == signature), IntentGuardError::BadSignature);
    require!(ix.data.windows(message.len()).any(|window| window == message), IntentGuardError::BadSignature);
    Ok(())
}

#[derive(Accounts)]
pub struct InitVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: protocol authority that will be represented by the vault PDA.
    pub authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + Vault::INIT_SPACE,
        seeds = [b"vault", authority.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct QueueProposal<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub vault: Account<'info, Vault>,
    #[account(
        init,
        payer = payer,
        space = 8 + Proposal::INIT_SPACE,
        seeds = [
            b"proposal",
            vault.key().as_ref(),
            &vault.nonce.to_le_bytes(),
        ],
        bump
    )]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: sysvar instruction account.
    #[account(address = INSTRUCTIONS_ID)]
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    pub signer: Signer<'info>,
    pub vault: Account<'info, Vault>,
    #[account(mut, has_one = vault)]
    pub proposal: Account<'info, Proposal>,
}

#[derive(Accounts)]
pub struct Execute<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut, has_one = vault)]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: protocol-specific adapter account.
    pub adapter: UncheckedAccount<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,
    pub threshold: u8,
    pub veto_threshold: u8,
    pub fresh_window_secs: i64,
    pub cooloff_secs: i64,
    pub execute_delay_secs: i64,
    pub min_proposal_lifetime_secs: i64,
    pub nonce: u64,
    #[max_len(MAX_SIGNERS)]
    pub signers: Vec<Pubkey>,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Proposal {
    pub vault: Pubkey,
    pub nonce: u64,
    pub target_program: Pubkey,
    pub adapter: Pubkey,
    pub intent_hash: [u8; 32],
    pub instruction_data_hash: [u8; 32],
    pub queued_at: i64,
    pub expires_at: i64,
    pub cancel_count: u8,
    #[max_len(MAX_CANCELLERS)]
    pub cancellers: Vec<Pubkey>,
    pub state: ProposalState,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SignerAttestation {
    pub signer: Pubkey,
    pub signed_at: i64,
    pub expires_at: i64,
    pub ed25519_instruction_index: u8,
    pub signature: [u8; 64],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ProposalState {
    Queued,
    Cancelled,
    Executed,
}

#[event]
pub struct VaultInitialized {
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub threshold: u8,
    pub veto_threshold: u8,
}

#[event]
pub struct ProposalQueued {
    pub proposal: Pubkey,
    pub vault: Pubkey,
    pub nonce: u64,
    pub target_program: Pubkey,
    pub intent_hash: [u8; 32],
    pub execute_after: i64,
    pub expires_at: i64,
}

#[event]
pub struct ProposalCancelled {
    pub proposal: Pubkey,
    pub signer: Pubkey,
    pub cancel_count: u8,
    pub reason: String,
}

#[event]
pub struct ProposalExecuted {
    pub proposal: Pubkey,
    pub target_program: Pubkey,
}

#[error_code]
pub enum IntentGuardError {
    #[msg("Bad vault or proposal config")]
    BadConfig,
    #[msg("Too many signers")]
    TooManySigners,
    #[msg("Duplicate signer")]
    DuplicateSigner,
    #[msg("Not enough valid signatures")]
    InsufficientSignatures,
    #[msg("Bad signature")]
    BadSignature,
    #[msg("Signature is not fresh")]
    SignatureNotFresh,
    #[msg("Proposal has expired")]
    ProposalExpired,
    #[msg("Proposal is in the wrong state")]
    BadState,
    #[msg("Signer is not part of the vault")]
    NotSigner,
    #[msg("Proposal nonce does not match vault nonce")]
    BadNonce,
    #[msg("Cool-off window is still active")]
    CooloffActive,
    #[msg("Instruction data does not match the queued proposal")]
    BadIntent,
    #[msg("Missing target program")]
    MissingTargetProgram,
    #[msg("Bad target program")]
    BadTarget,
    #[msg("Bad adapter")]
    BadAdapter,
}
