import { Injectable, OnModuleInit, Logger } from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { Keypair } from '@stellar/stellar-sdk';

@Injectable()
export class StellarKeypairService implements OnModuleInit {
  private readonly logger = new Logger(StellarKeypairService.name);
  private adminKeypair: Keypair;

  constructor(private configService: ConfigService) {}

  onModuleInit() {
    const secret = this.configService.get<string>('ADMIN_SECRET_KEY');
    if (secret && secret !== 'S...') {
      try {
        this.adminKeypair = Keypair.fromSecret(secret);
        this.logger.log(`Admin keypair loaded for public key: ${this.adminKeypair.publicKey()}`);
      } catch (error) {
        this.logger.error('Failed to load admin keypair from SECRET_KEY', error.stack);
      }
    } else {
      this.logger.warn('ADMIN_SECRET_KEY not set or using placeholder. Admin functions will be unavailable.');
    }
  }

  getAdminKeypair(): Keypair {
    if (!this.adminKeypair) {
      throw new Error('Admin keypair is not initialized. Check ADMIN_SECRET_KEY in .env');
    }
    return this.adminKeypair;
  }

  getAdminPublicKey(): string {
    return this.getAdminKeypair().publicKey();
  }
}
