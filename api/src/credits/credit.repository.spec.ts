import { InMemoryCreditRepository } from './credit.repository';
import { CreditEntity } from './credit.entity';
import { CreditStatus } from '../shared';

function makeCredit(id: string, projectId = 'PROJ-001'): CreditEntity {
  const e = new CreditEntity();
  e.id = id;
  e.projectId = projectId;
  e.issuer = 'GISSUER';
  e.vintageYear = 2024;
  e.methodology = 'VCS';
  e.geography = 'NG';
  e.tonnes = '1000000';
  e.ipfsHash = 'baf';
  e.status = CreditStatus.Active;
  e.issuedAt = 1700000000;
  return e;
}

describe('InMemoryCreditRepository', () => {
  let repo: InMemoryCreditRepository;

  beforeEach(() => {
    repo = new InMemoryCreditRepository();
  });

  it('saves and retrieves a credit by id', async () => {
    const credit = makeCredit('abc123');
    await repo.save(credit);
    const found = await repo.findById('abc123');
    expect(found).toEqual(credit);
  });

  it('returns undefined for unknown id', async () => {
    expect(await repo.findById('nope')).toBeUndefined();
  });

  it('paginates findAll correctly', async () => {
    for (let i = 0; i < 5; i++) await repo.save(makeCredit(`id${i}`));
    const page1 = await repo.findAll(1, 3);
    expect(page1.data).toHaveLength(3);
    expect(page1.total).toBe(5);
    const page2 = await repo.findAll(2, 3);
    expect(page2.data).toHaveLength(2);
  });

  it('filters by project', async () => {
    await repo.save(makeCredit('a', 'PROJ-001'));
    await repo.save(makeCredit('b', 'PROJ-002'));
    const result = await repo.findByProject('PROJ-001', 1, 10);
    expect(result.data).toHaveLength(1);
    expect(result.data[0].id).toBe('a');
  });
});
