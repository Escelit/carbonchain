import { Controller, Get, Param } from '@nestjs/common';
import { VerifiersService, VerifierInfo } from './verifiers.service';

@Controller('verifiers')
export class VerifiersController {
  constructor(private readonly verifiersService: VerifiersService) {}

  @Get()
  listVerifiers(): Promise<VerifierInfo[]> {
    return this.verifiersService.listVerifiers();
  }

  @Get(':address')
  getVerifier(@Param('address') address: string): Promise<VerifierInfo> {
    return this.verifiersService.getVerifier(address);
  }
}
