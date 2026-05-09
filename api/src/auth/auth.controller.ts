import {
  Controller,
  Post,
  Body,
  Get,
  Query,
  UseGuards,
  Request,
} from '@nestjs/common';
import { AuthService } from './auth.service';
import { AuthGuard } from '@nestjs/passport';

@Controller('auth')
export class AuthController {
  constructor(private readonly authService: AuthService) {}

  /** GET /auth/challenge?account=G... */
  @Get('challenge')
  getChallenge(@Query('account') account: string): {
    transaction: string;
    network_passphrase: string;
  } {
    return this.authService.generateChallenge(account);
  }

  /** POST /auth/token — verifies signed challenge, returns JWT */
  @Post('token')
  getToken(@Body() body: { transaction: string }): { access_token: string } {
    return this.authService.verifyAndIssueToken(body.transaction);
  }

  /** GET /auth/me — protected route, returns authenticated account */
  @UseGuards(AuthGuard('jwt'))
  @Get('me')
  getMe(@Request() req: { user: { account: string } }): { account: string } {
    return { account: req.user.account };
  }
}
