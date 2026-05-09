import { Controller, Get, Post, Param, Body } from '@nestjs/common';
import { CreditsService, IssueCreditDto } from './credits.service';
import { CreditMetadata } from '../shared';

@Controller('credits')
export class CreditsController {
  constructor(private readonly creditsService: CreditsService) {}

  @Post('issue')
  issueCredit(@Body() dto: IssueCreditDto): Promise<{ creditId: string }> {
    return this.creditsService.issueCredit(dto);
  }

  @Get(':id')
  async getCredit(@Param('id') id: string): Promise<CreditMetadata> {
    return this.creditsService.getCredit(id);
  }

  @Get('project/:projectId')
  async listByProject(
    @Param('projectId') projectId: string,
  ): Promise<string[]> {
    return this.creditsService.listCreditsByProject(projectId);
  }
}
