import {
  Controller,
  Get,
  Post,
  Delete,
  Param,
  Body,
  ParseIntPipe,
} from '@nestjs/common';
import { MarketplaceService, CreateOfferDto } from './marketplace.service';
import { Offer } from '../shared';

@Controller('marketplace')
export class MarketplaceController {
  constructor(private readonly marketplaceService: MarketplaceService) {}

  @Post('offer')
  createOffer(@Body() dto: CreateOfferDto): Promise<{ offerId: string }> {
    return this.marketplaceService.createOffer(dto);
  }

  @Get('offer/:id')
  getOffer(@Param('id', ParseIntPipe) id: number): Promise<Offer> {
    return this.marketplaceService.getOffer(id);
  }

  @Get('seller/:address')
  getOffersBySeller(@Param('address') address: string): Promise<string[]> {
    return this.marketplaceService.getOffersBySeller(address);
  }

  @Delete('offer/:id/seller/:address')
  cancelOffer(
    @Param('address') address: string,
    @Param('id', ParseIntPipe) id: number,
  ): Promise<void> {
    return this.marketplaceService.cancelOffer(address, id);
  }
}
