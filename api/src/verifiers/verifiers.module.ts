import { Module } from '@nestjs/common';
import { ConfigModule } from '@nestjs/config';
import { VerifiersService } from './verifiers.service';
import { VerifiersController } from './verifiers.controller';
import { StellarModule } from '../stellar/stellar.module';

@Module({
  imports: [ConfigModule, StellarModule],
  controllers: [VerifiersController],
  providers: [VerifiersService],
  exports: [VerifiersService],
})
export class VerifiersModule {}
