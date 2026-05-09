import { Injectable, Logger, NotFoundException } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { StellarService } from '../stellar/stellar.service';
import { scValToNative } from '@stellar/stellar-sdk';

export interface VerifierInfo {
  address: string;
}

@Injectable()
export class VerifiersService {
  private readonly logger = new Logger(VerifiersService.name);
  private readonly contractId: string;

  constructor(
    private readonly stellarService: StellarService,
    private readonly configService: ConfigService,
  ) {
    this.contractId = this.configService.get<string>(
      'CREDIT_REGISTRY_CONTRACT_ID',
      '',
    );
  }

  async listVerifiers(): Promise<VerifierInfo[]> {
    try {
      const retval = await this.stellarService.readContract(
        this.contractId,
        'list_verifiers',
      );
      if (!retval) return [];
      const native = scValToNative(retval) as string[];
      return native.map((address) => ({ address }));
    } catch (err: unknown) {
      this.logger.error(`Failed to list verifiers: ${(err as Error).message}`);
      return [];
    }
  }

  async getVerifier(address: string): Promise<VerifierInfo> {
    const verifiers = await this.listVerifiers();
    const found = verifiers.find((v) => v.address === address);
    if (!found) throw new NotFoundException(`Verifier ${address} not found`);
    return found;
  }
}
