import { Controller, Post, Get, Param, Body, UseGuards, Query, ParseIntPipe, DefaultValuePipe } from '@nestjs/common';
import { RetirementService, RetireDto } from './retirement.service';
import { RetirementRecord } from '../shared';
import { JwtAuthGuard } from '../auth/jwt-auth.guard';
import { PageResult } from '../credits/credit.repository';

@Controller('retirement')
export class RetirementController {
  constructor(private readonly retirementService: RetirementService) {}

  /** POST /retirement — protected: requires JWT */
  @UseGuards(JwtAuthGuard)
  @Post()
  retire(@Body() dto: RetireDto): Promise<{ retirementId: string }> {
    return this.retirementService.retire(dto);
  }

  /** GET /retirement — paginated list */
  @Get()
  listRetirements(
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit: number,
  ): Promise<PageResult<RetirementRecord>> {
    return this.retirementService.listRetirements(page, limit);
  }

  /** GET /retirement/:id — fetch a retirement record */
  @Get(':id')
  getRetirement(@Param('id') id: string): Promise<RetirementRecord> {
    return this.retirementService.getRetirement(id);
  }

  /** GET /retirement/account/:address — paginated retirements for an account */
  @Get('account/:address')
  getByAccount(
    @Param('address') address: string,
    @Query('page', new DefaultValuePipe(1), ParseIntPipe) page: number,
    @Query('limit', new DefaultValuePipe(20), ParseIntPipe) limit: number,
  ): Promise<PageResult<RetirementRecord>> {
    return this.retirementService.getRetirementsByAccount(address, page, limit);
  }
}
