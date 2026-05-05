import { Module } from '@nestjs/common';
import { ConfigModule } from '@nestjs/config';
import { StellarService } from './stellar.service';
import { StellarKeypairService } from './stellar-keypair.service';

@Module({
  imports: [ConfigModule],
  providers: [StellarService, StellarKeypairService],
  exports: [StellarService, StellarKeypairService],
})
export class StellarModule {}
