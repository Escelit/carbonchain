import { Controller, Get, Post, Body, Param } from '@nestjs/common';
import { ProjectsService } from './projects.service';
import type { ProjectProfile } from '../shared';

@Controller('projects')
export class ProjectsController {
  constructor(private readonly projectsService: ProjectsService) {}

  @Post()
  create(@Body() data: Omit<ProjectProfile, 'id'>): ProjectProfile {
    return this.projectsService.createProject(data);
  }

  @Get(':id')
  getOne(@Param('id') id: string): ProjectProfile {
    return this.projectsService.getProject(id);
  }

  @Get()
  list(): ProjectProfile[] {
    return this.projectsService.listProjects();
  }
}
