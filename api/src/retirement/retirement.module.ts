import { Module } from '@nestjs/common';
import { ConfigModule } from '@nestjs/config';
import { RetirementService } from './retirement.service';
import { RetirementController } from './retirement.controller';
import { StellarModule } from '../stellar/stellar.module';
import { AuthModule } from '../auth/auth.module';
import { InMemoryRetirementRepository, RETIREMENT_REPOSITORY } from './retirement.repository';

@Module({
  imports: [ConfigModule, StellarModule, AuthModule],
  controllers: [RetirementController],
  providers: [
    RetirementService,
    { provide: RETIREMENT_REPOSITORY, useClass: InMemoryRetirementRepository },
  ],
  exports: [RetirementService],
})
export class RetirementModule {}
